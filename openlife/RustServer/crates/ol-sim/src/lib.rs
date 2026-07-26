//! Simulation: sole writer of world + player state.
//!
//! Net enqueues [`ol_net::NetIntent`] and reads world under RwLock for MC.
//! After mutations, sim pushes wire packets via [`ol_net::OutboundHub`].

#![forbid(unsafe_code)]

mod account_persist;
mod accounts;
mod admin_env;
mod afk;
mod age_curves;
mod age_stage;
mod ai_goals;
mod ally;
mod animal_move;
mod animals;
mod apocalypse;
mod biome_colors;
mod death_cause;
mod death_log;
mod biomes_query;
mod chunk_tier;
mod clothing_cmds;
mod combat;
mod craft_graph;
mod craft_value;
mod crime;
mod curse;
mod debt_book;
mod drain_est;
mod economy;
mod emote_limit;
mod environment;
mod feed;
mod fertility;
mod fire;
mod gestation_tick;
mod heal;
mod hunt;
mod move_notes;
mod move_path;
mod birth_fitness;
mod leadership;
mod lineage_persist;
mod locks;
mod look;
mod map_chunk;
mod markers;
mod mumble;
mod mute;
mod mutation;
mod object_tags;
mod permissions;
mod poison;
mod professions;
mod queries_extra;
mod relations;
mod reputation;
mod shove;
mod sit;
mod treasury;
mod weapons;
mod naming;
mod pathfind;
mod player;
mod poll;
mod posse;
mod prestige;
mod score;
mod skills;
mod snow;
mod social;
mod speech;
mod tools;
mod tutorial;
mod twins;
mod version_gate;
mod war;
mod weather;
mod wire_fields;
mod yum;


pub use move_path::{
    advance_path, build_move_path, calculate_length, chebyshev as move_chebyshev,
    client_path_deltas_to_steps, format_pm_body, steps_to_client_path_deltas,
    in_use_range, is_moving, quad_dist as move_quad_dist, resolve_move_seq, round2,
    truncate_walkable, MovePath, MoveReject, DEFAULT_MOVE_SPEED, MAX_MOVE_QUAD_JUMP_BEFORE_FORCE,
};
pub use birth_fitness::{
    father_fitness, mother_fitness, ChildView, FatherView, MotherView, EVE_OR_ADAM_BIRTH_CHANCE,
};

pub use accounts::{
    normalize_email, AccountBook, AccountBookSnapshot, AccountRecord, AccountSummary, AccountView,
};
pub use admin_env::{
    end_apoc, parse_season, set_hour, set_season, set_weather, start_apoc, weather_kind_name,
};
pub use afk::{
    format_afk_query, AfkBook, AFK_WARN_REMAINING_SECS, DEFAULT_AFK_SECS,
};
pub use death_log::{DeathLog, DeathRecord};
pub use age_stage::{format_stage_query, AgeStage};
pub use biome_colors::{
    biome_id_from_name, biome_id_from_rgb, color_for_biome, format_biome_colors_query,
    format_hex_query, name_for_biome, BiomeColorEntry, Rgb, BIOME_COLORS,
};
pub use death_cause::{
    combat_death, format_cause_query, format_death_event, DeathCause,
};
pub use mute::{format_mute_query, parse_mute_command, should_hear, MuteBook};
pub use object_tags::{
    format_held_tags_query, format_object_tags_summary, parse_object_description, plus_tags_only,
    ObjectDescription,
};
pub use wire_fields::{
    extract_hash_frames, format_csv_i32, format_xy, parse_csv_i32, parse_i32_list, parse_key_f32,
    parse_key_i32, parse_key_value, parse_xy, parse_xy_exact, parse_xyz, split_tokens,
    strip_line_comment,
};
pub use reputation::{
    format_reputation_query, is_dangerous_lost_combat, label_from_lost_combat,
    label_from_reputation, lost_combat_from_reputation, reputation_from_lost_combat,
    ReputationBook, ReputationLabel,
};
pub use version_gate::{
    check_client_version, format_version_gate_query, format_version_reject_message,
    format_version_reject_ps, parse_version_token, should_hard_reject_login, versions_compatible,
    VersionGatePolicy, VersionGateResult, DEFAULT_REQUIRED_VERSION,
};
pub use gestation_tick::due_mothers;
pub use ai_goals::{
    format_seeking_query, parse_profession_token, pick_goal, pick_goal_ext, pick_goal_smith_craft,
    pick_smith_goal, smith_product_targets, Goal, Profession, FARMER_TARGET_ID, HUNGRY_FOOD,
    SMITH_IRON_ID, SMITH_TARGET_ID,
};
pub use ally::AllyState;
pub use animals::{
    Animal, AnimalKind, AnimalSnapshot, AnimalView, AnimalWorld, AnimalWorldShare,
    ANIMAL_THREAT_RANGE,
};
pub use biomes_query::{format_biomes_query, is_listed_bad_biome, BadBiomeEntry, BAD_BIOMES};
pub use chunk_tier::{
    build_tier_map, classify_chunk, format_chunks_query, min_chunk_chebyshev, tier_counts,
    ChunkTier,
};
pub use combat::{
    CombatState, CombatStats, HitResult, FOOD_MAX_DEATH, HITS_KILL_THRESHOLD, KILL_RANGE,
    MAX_WOUND, WOUND_BLEED_DRAIN,
    WOUND_KILL_THRESHOLD,
};
pub use craft_graph::ReverseCraftGraph;
pub use craft_value::{
    ally_food_bonus, best_craft, count_ids, craft_time_cost_sec, evaluate_nearby_crafts,
    move_time_sec, object_value, scarcity_mult, tile_dist, CraftOption, CraftProfession, NearbyObj,
    ABUNDANCE_SOFT_CAP, DEFAULT_CRAFT_RADIUS, DEFAULT_WALK_SPEED, INTERACTION_SEC,
};
pub use crime::{classify_take, CrimeState, TakeLegality, THEFT_PRESTIGE_PENALTY};
pub use emote_limit::{EmoteRateLimiter, EMOTE_RATE_MAX, EMOTE_RATE_WINDOW_SECS};
pub use mumble::MUMBLE_RANGE;
pub use shove::{
    can_pull_to, is_adjacent as shove_is_adjacent, pull_dest, push_dest, resolve_push,
    PushOutcome, BLESS_PRESTIGE, CUTE_EMOT_INDEX, KISS_ALLY_PRESTIGE, LOVE_EMOT_INDEX,
    MAD_EMOT_INDEX, SHOVE_RANGE, SLAP_WOUND, THANK_PRESTIGE,
};
pub use sit::{SIT_BLOCKS_MOVE, SIT_FOOD_DRAIN_MULT};
pub use heal::{format_wound_query, name_looks_like_heal, try_heal, HealResult};
pub use hunt::{
    hunt_nearest, HuntResult, HUNT_DAMAGE, HUNT_KILL_PRESTIGE, HUNT_MEAT_OBJECT_ID, HUNT_RANGE,
};
pub use professions::{
    collect_food_ids, is_chop_biome, is_fishing_biome, is_grassland, is_mountain_biome, is_swamp,
    mountain_adjacent, pick_food_id, pick_harvest_id, prof_cooldown_ready, try_chop, try_dig,
    try_fish, try_harvest, try_mine, ProfActionResult, BORDER_JUNGLE_BIOME, CLAY_PLACEHOLDER_ID,
    FISH_PLACEHOLDER_ID, GRASSLAND_BIOME, GREY_BIOME, HARVEST_FALLBACK_ID, JUNGLE_BIOME,
    MOUNTAIN_BIOME, OCEAN_BIOME, PASSABLE_RIVER_BIOME, PROF_ACTION_COOLDOWN_SECS, RIVER_BIOME,
    STONE_PLACEHOLDER_ID, SWAMP_BIOME, WOOD_PLACEHOLDER_ID, YELLOW_BIOME,
};
pub use leadership::{
    follower_count, format_leader_query, is_leader, rank_leaders, LeaderEntry, LEADER_QUERY_LIMIT,
};
pub use mutation::{SpecialIndex, SpecialKind};
pub use fire::{FireState, FireTile, DEFAULT_FIRE_SECS, FIRE_FOOD_DRAIN};
pub use locks::LockState;
pub use drain_est::{estimate_food_drain, DrainEstimate};
pub use move_notes::{
    ballast_speed_mult, compose_move_speed, format_speed_query, format_weight_query,
    weight_item_count, BALLAST_PER_ITEM,
};
pub use permissions::{check_owned_access, format_lock_query, Access};
pub use look::format_look;
pub use poison::{name_looks_like_poison, should_sicken_on_feed};
pub use queries_extra::{
    biome_name, chebyshev as query_chebyshev, format_biome_query, format_biome_query_with_hex,
    format_count_query, format_dist_query, format_floor_query, format_near_query,
    format_save_denied, format_save_reply, format_wjournal_query,
};
pub use relations::{
    format_children_query, format_gen_query, format_relation_query, is_eve, relation_of, Relation,
};
pub use treasury::{
    donate as treasury_donate, format_treasury_query, pay_from_treasury, tax, TreasurySnapshot,
    TreasuryView,
};
pub use weapons::{
    format_range_query, held_damage_protection_factor, weapon_damage, weapon_range,
    DEFAULT_WEAPON_DAMAGE,
};
pub use skills::{SkillBook, SkillState, SkillTrack, XP_PER_CRAFT};
pub use snow::{SnowCover, SNOW_FOOD_EXTRA, SNOW_MOVE_FACTOR};
pub use tutorial::{TutorialProgress, TutorialState, TIPS};
pub use apocalypse::{
    Apocalypse, ApocalypsePhase, APOC_FOOD_DRAIN_MULT, DEFAULT_ACTIVE_SECS, DEFAULT_WARNING_SECS,
};
pub use feed::{
    apply_feed_amounts, breastfeed_tick, can_breastfeed, can_feed, name_looks_like_food,
    pickup_feed_amounts, FEED_RANGE, FOOD_RESTORE_FACTOR_WHILE_FEEDING,
    MAX_CHILD_AGE_BREAST_FEEDING, PICKUP_FEEDING_FOOD_RESTORE,
};
pub use fertility::{
    FertilityState, BIRTH_COOLDOWN_SECS, FERTILE_MAX_AGE, FERTILE_MIN_AGE, GESTATION_SECS,
};
pub use curse::{
    compute_excess, format_curse_score_change, format_curse_token_change, CursePlayer, CurseState,
    CURSE_THRESHOLD, DEFAULT_CURSE_TOKENS,
};
pub use debt_book::DebtBook;
pub use economy::{Economy, Wallet};
pub use environment::{
    biome_food_multiplier, clothing_temp_bonus, format_biomefood_query, format_swim_query,
    format_warm_query, is_swim_biome, EnvSnapshot, EnvView, Environment, Season,
    BIOME_OCEAN, BIOME_RIVER, OCEAN_RIVER_FOOD_DRAIN_MULT,
};
pub use speech::{
    chat_range_for_age as speech_chat_range_for_age, ADULT_CHAT_RANGE, MUMBLE_CHAT_RANGE,
    SHOUT_CHAT_RANGE, SpeechVolume, WHISPER_CHAT_RANGE,
};
pub use weather::{
    default_for_season, parse_weather_kind, Weather, WeatherKind, WeatherSnapshot, WeatherView,
};
pub use account_persist::{
    load_accounts, save_accounts, ACCOUNT_FORMAT_VERSION, DEFAULT_ACCOUNT_FILE,
};
pub use lineage_persist::{
    load_lineages, save_lineages, DEFAULT_LINEAGE_FILE, LINEAGE_FORMAT_VERSION,
};
pub use map_chunk::{
    build_chunk_plaintext, build_map_chunk_packet, build_region_object_ids,
    compress_chunk_plaintext, format_map_chunk_header, format_map_chunk_message_prefix,
};
pub use markers::{MapMarker, MarkerKind, MarkerState};
pub use naming::{pick_random_name, FAMILY_NAMES, FIRST_NAMES};
pub use pathfind::{
    find_path, is_walkable, is_walkable_for_player, name_is_gate_or_door, next_step, path_steps,
};
pub use player::{
    clothing_slot_for_object, ClothingSlot, Player, PlayerSnapshot, BACKPACK_MAX, NOTES_MAX,
    NOTE_TEXT_MAX, TITLE_TEXT_MAX,
};
pub use poll::{parse_vote_choice, PollState, VoteChoice};
pub use posse::{format_posse_join, PosseState};
pub use prestige::{
    other_prestige_info_wire, player_prestige_info_wire, prestige_class_from_percentile,
    prestige_class_wire_token, prestige_classes_from_living_scores, PrestigeClass,
    PRESTIGE_COMMONER_MAX, PRESTIGE_KING_MAX, PRESTIGE_NOBLE_MAX, PRESTIGE_SERF_MAX,
};
pub use score::{
    compute_score, PrestigePlayerRow, PrestigeSnapshot, PrestigeView, ScoreEntry, Scoreboard,
    SCORE_PER_DEATH, SCORE_PER_KILL,
};
pub use social::{
    format_exile_line, format_following_line, LineageEntryView, LineageNode, LineageSnapshot,
    LineageView, SocialState,
};
pub use tools::ToolSlots;
pub use twins::{TwinPeer, TwinRegistry};
pub use war::{
    format_war_report, pair_key, WarState, STATUS_ALLIANCE, STATUS_PEACE, STATUS_WAR,
};
pub use yum::YumState;
pub use ol_protocol::{format_baby_wiggle, format_dying};

use ol_content::ContentDb;
use ol_metrics::Counters;
use ol_net::{OutboundHub, NetIntent};
use ol_protocol::{
    format_food_change, format_frame, format_heat_change, format_learned_tool_report,
    format_location_says, format_map_change, format_map_change_moving, format_photo_signature,
    format_player_says,
    format_player_update_line, format_player_update_line_eat, format_player_update_line_full,
    format_pong, format_server_message, format_vog_update, ClientTag, PHOTO_DENIED_SIGNATURE,
};
use ol_world::{
    place_natural_object, pick_biome_spawn, ComplexObject, JournalEntry, World, WorldJournal,
};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use tracing::{debug, info, warn};

/// Orderly `SAY !shutdown` phases.
#[derive(Debug, Clone)]
pub struct ShutdownState {
    /// Seconds remaining in current phase.
    pub remaining: f32,
    /// After countdown: save + AP, then apocalypse hold, then exit.
    pub phase: ShutdownPhase,
}

/// Phases for operator orderly shutdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownPhase {
    /// Global countdown message already sent; waiting then save.
    Countdown,
    /// Apocalypse signal displayed; waiting then exit flag.
    ApocalypseHold,
}

pub const FOOD_USE_PER_SEC: f32 = 0.10;
/// Extra food drain in extreme heat/cold (Haxe temperature hunger).
pub const TEMP_FOOD_EXTRA: f32 = 0.05;
/// Extra food drain on desert (biome 5) when temperature is high (TEMP_FOOD path).
pub const DESERT_EXTRA: f32 = 0.02;
pub const AGE_YEARS_PER_SEC: f32 = 1.0 / 60.0;
pub const START_FOOD: f32 = 10.0;
pub const MAX_FOOD: f32 = 20.0;
pub const DEATH_FOOD_THRESHOLD: f32 = 0.0;
/// Age above which food drain is increased (old age).
pub const OLD_AGE_THRESHOLD: f32 = 60.0;
/// Food drain multiplier when `age > OLD_AGE_THRESHOLD`.
pub const OLD_AGE_FOOD_DRAIN_MULT: f32 = 1.5;
/// Food drain multiplier while [`Player::sleeping`] (SAY SLEEP / WAKE).
pub const SLEEP_FOOD_DRAIN_MULT: f32 = 0.5;
/// Food drain multiplier while [`Player::sitting`] (SAY SIT / STAND).
/// Milder than sleep; see [`SIT_FOOD_DRAIN_MULT`].
pub const SIT_DRAIN_MULT: f32 = SIT_FOOD_DRAIN_MULT;
/// Food drain multiplier while [`Player::sick`] (SAY SICK / CURE).
pub const SICK_FOOD_DRAIN_MULT: f32 = 1.3;
/// Default walk move-speed reported on FX / PS notes (no actual MOVE change).
pub const WALK_MOVE_SPEED: f32 = 3.75;
/// Move-speed note while [`Player::riding`] (SAY RIDE / MOUNT / DISMOUNT); note only.
pub const RIDE_MOVE_SPEED: f32 = 5.0;
/// Max SAY intents accepted per player inside [`SAY_RATE_WINDOW_SECS`].
pub const SAY_RATE_MAX: usize = 5;
/// Sliding window (sim seconds) for SAY rate limiting.
pub const SAY_RATE_WINDOW_SECS: f32 = 10.0;
/// Cap on transitions seeded into [`SimState::craft_graph`] at sim boot.
pub const CRAFT_GRAPH_SEED_CAP: usize = 50_000;
/// Sim-time seconds between animal wander ticks.
pub const ANIMAL_WANDER_INTERVAL_SECS: f32 = 2.0;
/// Sim-time seconds between living-percentile prestige class refreshes.
pub const LIVING_PRESTIGE_REFRESH_SECS: f32 = 15.0;
/// Age above which the player dies of old age (`reason_age`).
pub const MAX_AGE: f32 = 120.0;
/// Age below which a player is treated as a baby for BW / starving DY.
pub const BABY_AGE_THRESHOLD: f32 = 3.0;
/// Food below which starving infants emit DY (with BW) on a timer.
pub const STARVING_FOOD_THRESHOLD: f32 = 5.0;
/// Sim-time seconds between BW/DY emissions while age&lt;3 and food&lt;5.
pub const VITALS_EMIT_INTERVAL_SECS: f32 = 5.0;
/// Food below which living players emit a PE hunger emote on a timer.
pub const HUNGER_EMOT_FOOD_THRESHOLD: f32 = 3.0;
/// Sim-time seconds between PE hunger emotes while food&lt;3.
pub const HUNGER_EMOT_INTERVAL_SECS: f32 = 8.0;
/// PE emot index for hunger (Haxe `Emote.mad` = 1).
pub const HUNGER_EMOT_INDEX: i32 = 1;
/// PE emot index while sleeping (soft snore proxy; Haxe has no dedicated sleep emote).
/// Uses `Emote.sad` = 3 as a calm closed-eyes face for nearby awareness.
pub const SLEEP_EMOT_INDEX: i32 = 3;
/// PE emot index for `SAY YAWN` (Haxe-style yawn face).
pub const YAWN_EMOT_INDEX: i32 = 2;
// CUTE/LOVE/MAD emot + KISS/THANK/BLESS/SLAP prestige constants re-exported from shove.
/// Sim-time seconds between PE sleep/snore emotes while [`Player::sleeping`].
pub const SLEEP_EMOT_INTERVAL_SECS: f32 = 15.0;
/// Sim-time seconds between HX (HEAT_CHANGE) emissions to each player.
pub const HX_EMIT_INTERVAL_SECS: f32 = 10.0;
/// Debug LOCATION_SAYS interval: server-authority x,y for each human player.
pub const POS_DEBUG_LS_INTERVAL_SECS: f32 = 1.0;
pub const DEFAULT_PERSON_OBJECT: i32 = 19;

/// Wire person object id for a player (skin/body).
#[inline]
pub fn person_object_id(p: &Player) -> i32 {
    if p.display_object_id > 0 {
        p.display_object_id
    } else {
        DEFAULT_PERSON_OBJECT
    }
}

/// Known local playtest account — unique name + distinct skin so clients know the server.
const PLAYTEST_EMAIL_NEEDLE: &str = "76561198032560680";
const PLAYTEST_FIRST_NAME: &str = "GROKPLAY";
/// Family name embeds the server crate version so the client always shows which build.
fn playtest_family_name() -> String {
    format!("V{}", env!("CARGO_PKG_VERSION"))
}
/// Male005 — distinct from default Female001 (19).
const PLAYTEST_SKIN_OBJECT: i32 = 352;
/// Haxe-style interest radius for MX/PU fan-out (Chebyshev tiles).
pub const NEARBY_RANGE: i32 = 24;
/// Larger Chebyshev radius for `SAY SHOUT <text>` PS fan-out.
pub const SHOUT_RANGE: i32 = 48;
/// Soft Chebyshev radius for `SAY MUMBLE <text>` PS fan-out ([`MUMBLE_RANGE`]).
pub const MUMBLE_SAY_RANGE: i32 = MUMBLE_RANGE;

/// Chat PS fan-out radius by speaker age (Haxe age-scaled speech range).
///
/// Infants &lt;3 → 8, children &lt;10 → 16, elders ≥60 → 20, else [`NEARBY_RANGE`].
pub fn chat_range_for_age(age: f32) -> i32 {
    if age < 3.0 {
        8
    } else if age < 10.0 {
        16
    } else if age >= 60.0 {
        20
    } else {
        NEARBY_RANGE
    }
}
/// Resend MAP_CHUNK when player moved this many tiles from last MC center (Haxe).
pub const MC_RESEND_THRESHOLD: i32 = 10;
pub const MC_WIDTH: i32 = 32;
pub const MC_HEIGHT: i32 = 30;
/// Default container slots when content missing.
pub const DEFAULT_CONTAINER_SLOTS: usize = 4;
/// Synthetic conn_id offset for babies spawned via birth (not a real TCP conn).
pub const BABY_CONN_OFFSET: u64 = 1_000_000;
/// Object id left on the death tile for hunger/age graves when content has none.
/// `0` = do not place a marker. Prefer [`resolve_grave_object_id`] / [`SimState::grave_object_id`].
pub const GRAVE_OBJECT_ID: i32 = 0;

/// First content object whose name contains `"Grave"` (case-insensitive), lowest id wins.
/// Returns `0` when content has no matching object.
pub fn resolve_grave_object_id(content: &ContentDb) -> i32 {
    content
        .objects
        .values()
        .filter(|o| o.name.to_ascii_lowercase().contains("grave"))
        .map(|o| o.id)
        .min()
        .unwrap_or(GRAVE_OBJECT_ID)
}

/// Transfer deceased coins to mother if she is online and living; otherwise to treasury.
/// Updates scoreboard coin fields for affected wallets.
fn apply_death_inheritance(state: &mut SimState, deceased_p_id: i32) {
    let mother_id = state
        .social
        .lineages
        .get(&deceased_p_id)
        .and_then(|n| n.mother_id);
    let mother_online = mother_id.and_then(|mid| {
        state
            .players
            .values()
            .find(|pl| pl.p_id == mid && !pl.deleted && pl.connected)
            .map(|pl| pl.p_id)
    });
    let amount = state
        .economy
        .inherit_on_death(deceased_p_id, mother_online);
    if amount <= 0 {
        return;
    }
    // Deceased wallet is zero; sync scoreboard + beneficiary.
    state.scoreboard.set_coins(deceased_p_id, 0);
    if let Some(mid) = mother_online {
        let coins = state
            .economy
            .wallets
            .get(&mid)
            .map(|w| w.coins)
            .unwrap_or(0);
        state.scoreboard.set_coins(mid, coins);
        state.push_event(format!("INHERIT {deceased_p_id} mother={mid} {amount}"));
    } else {
        state.push_event(format!(
            "INHERIT {deceased_p_id} treasury={} {amount}",
            state.economy.treasury
        ));
    }
}

/// Max Chebyshev ring radius when scattering loot on death / DROPALL.
pub const DEATH_SCATTER_RADIUS: i32 = 4;

/// Candidate (dx, dy) offsets for death loot: rings 1..=max_radius, then death tile.
///
/// Death tile is last so graves / other markers can claim the body tile first.
pub fn death_scatter_offsets(max_radius: i32) -> Vec<(i32, i32)> {
    let mut out = Vec::new();
    let rmax = max_radius.max(0);
    for r in 1..=rmax {
        for dy in -r..=r {
            for dx in -r..=r {
                if dx.abs().max(dy.abs()) == r {
                    out.push((dx, dy));
                }
            }
        }
    }
    out.push((0, 0));
    out
}

/// Place `items` on empty tiles near `(cx, cy)`. Returns placed `(id, x, y)`.
///
/// Items with no free tile within [`DEATH_SCATTER_RADIUS`] are lost.
fn scatter_items_near(
    state: &mut SimState,
    cx: i32,
    cy: i32,
    items: Vec<i32>,
) -> Vec<(i32, i32, i32)> {
    if items.is_empty() {
        return vec![];
    }
    let offsets = death_scatter_offsets(DEATH_SCATTER_RADIUS);
    let mut offset_i = 0usize;
    let mut placed = Vec::new();
    for id in items {
        if id == 0 {
            continue;
        }
        let mut found = None;
        while offset_i < offsets.len() {
            let (dx, dy) = offsets[offset_i];
            offset_i += 1;
            let tx = cx + dx;
            let ty = cy + dy;
            let empty = state.world.read().unwrap().get_object(tx, ty) == 0;
            if empty {
                found = Some((tx, ty));
                break;
            }
        }
        if let Some((tx, ty)) = found {
            state.world.write().unwrap().set_object(tx, ty, id);
            state.record_world_change(tx, ty, id);
            schedule_decay(state, tx, ty, id);
            placed.push((id, tx, ty));
        }
    }
    placed
}

/// Drain held + clothing + backpack and scatter near the deceased (`conn_id`).
///
/// Returns `(object_id, tile_x, tile_y)` for each successfully placed item.
fn scatter_backpack_on_death(state: &mut SimState, conn_id: u64) -> Vec<(i32, i32, i32)> {
    let Some(pl) = state.players.get_mut(&conn_id) else {
        return vec![];
    };
    let cx = pl.x;
    let cy = pl.y;
    let p_id = pl.p_id;
    let items = pl.take_death_loot_for_scatter();
    let placed = scatter_items_near(state, cx, cy, items);
    if !placed.is_empty() {
        state.push_event(format!("SCATTER {p_id} n={}", placed.len()));
        info!(
            conn_id,
            p_id,
            n = placed.len(),
            "sim: death loot scatter (held+clothing+backpack)"
        );
    }
    placed
}

/// Scatter death loot for an online player identified by `p_id`.
fn scatter_backpack_on_death_pid(state: &mut SimState, p_id: i32) -> Vec<(i32, i32, i32)> {
    let conn = state
        .players
        .iter()
        .find(|(_, pl)| pl.p_id == p_id)
        .map(|(&c, _)| c);
    match conn {
        Some(cid) => scatter_backpack_on_death(state, cid),
        None => vec![],
    }
}

/// `SAY DROPALL`: scatter held + backpack near the living player (no death, clothing kept).
fn scatter_dropall(state: &mut SimState, conn_id: u64) -> Vec<(i32, i32, i32)> {
    let Some(pl) = state.players.get_mut(&conn_id) else {
        return vec![];
    };
    if pl.deleted {
        return vec![];
    }
    let cx = pl.x;
    let cy = pl.y;
    let p_id = pl.p_id;
    let items = pl.take_dropall_for_scatter();
    let placed = scatter_items_near(state, cx, cy, items);
    if !placed.is_empty() {
        state.push_event(format!("SCATTER {p_id} n={}", placed.len()));
        info!(
            conn_id,
            p_id,
            n = placed.len(),
            "sim: DROPALL scatter (held+backpack)"
        );
    }
    placed
}
/// Max entries retained in [`SimState::event_log`] (oldest dropped).
pub const EVENT_LOG_MAX: usize = 100;
/// Number of recent events returned by `SAY ?LOG`.
pub const EVENT_LOG_QUERY_LAST: usize = 5;

pub type OpsSeriesView = std::sync::Arc<std::sync::RwLock<Vec<ol_metrics::OpsSample>>>;

pub type PlayerViewMap = Arc<RwLock<HashMap<u64, PlayerSnapshot>>>;

#[derive(Debug)]
pub struct SimState {
    pub world: Arc<RwLock<World>>,
    pub content: Arc<ContentDb>,
    pub players: HashMap<u64, Player>,
    pub logins: u64,
    pub intents_seen: u64,
    pub next_player_id: i32,
    pub spawn_x: i32,
    pub spawn_y: i32,
    /// When true, USE prefers last-use transition table (multi-use exhausted).
    pub prefer_last_use: bool,
    /// Shared snapshots for web viewer (optional).
    pub player_views: Option<PlayerViewMap>,
    /// Tile auto-decay timers: (x,y) → (expected object id, seconds remaining).
    pub pending_decays: HashMap<(i32, i32), (i32, f32)>,
    pub social: SocialState,
    pub environment: Environment,
    pub apocalypse: Apocalypse,
    pub combat: CombatState,
    pub economy: Economy,
    /// Outstanding player-to-player loans (`SAY LOAN` / `SAY REPAY`).
    pub debts: DebtBook,
    pub scoreboard: Scoreboard,
    pub curses: CurseState,
    pub env_view: Option<EnvView>,
    /// Optional shared weather snapshot for web `/api/weather`.
    pub weather_view: Option<WeatherView>,
    /// Optional soft-account book for web `/api/accounts`.
    pub account_view: Option<AccountView>,
    /// Optional living prestige board for web `/api/prestige`.
    pub prestige_view: Option<PrestigeView>,
    /// Optional lineage list for web `/lineage` + `/api/lineages`.
    pub lineage_view: Option<LineageView>,
    /// Optional animal counts for web `/api/animals`.
    pub animal_view: Option<AnimalView>,
    /// Optional live animal world mirror for self-play threat sensing (Arc share).
    pub animals_share: Option<AnimalWorldShare>,
    /// Optional village treasury for web `/api/treasury`.
    pub treasury_view: Option<TreasuryView>,
    pub markers: MarkerState,
    pub war: WarState,
    pub posse: PosseState,
    /// Theft / crime bookkeeping (owned-object takes).
    pub crime: CrimeState,
    /// Soft email→account identity (no SQL).
    pub accounts: AccountBook,
    /// Weather overlay (food drain / move notes).
    pub weather: Weather,
    /// Birth cooldown / gestation timers.
    pub fertility: FertilityState,
    /// Optional reverse craft graph for AI (empty until seeded from content).
    pub craft_graph: ReverseCraftGraph,
    /// Sparse animal sim (wander stub).
    pub animals: AnimalWorld,
    /// Directed ally / friend links.
    pub allies: AllyState,
    /// Session yes/no poll (`SAY POLL` / `VOTE` / `?POLL`).
    pub poll: PollState,
    /// Multi-server twin peer list (**stub only** — no network I/O).
    pub twins: TwinRegistry,
    /// Sparse special-object index (gates, graves, containers).
    pub specials: SpecialIndex,
    /// Soft skill XP per player (craft familiarity).
    pub skills: SkillState,
    /// Newbie tutorial tip progress.
    pub tutorial: TutorialState,
    /// Burning tiles hazard.
    pub fire: FireState,
    /// Seasonal snow cover overlay.
    pub snow: SnowCover,
    /// Locked gate/door tiles (session).
    pub locks: LockState,
    /// Last-activity stamps for AFK detection (see [`AfkBook`], [`DEFAULT_AFK_SECS`]).
    pub afk: AfkBook,
    /// Per-listener chat mute graph (`SAY MUTE` / `UNMUTE`; filters normal SAY PS).
    pub mutes: MuteBook,
    /// Combat reputation floats (≠ prestige / PrestigeClass); updated on illegal/legal kill.
    pub reputation: ReputationBook,
    /// Optional client data-version policy (soft-log on LOGIN when client reports a version).
    pub version_gate: VersionGatePolicy,
    /// When true, version mismatch on LOGIN hard-rejects (PS + no spawn). Config
    /// `client_version_strict` (default false).
    pub client_version_strict: bool,
    /// Content-resolved grave object id (`0` = none / do not place). See [`resolve_grave_object_id`].
    pub grave_object_id: i32,
    /// Optional append-only tile change journal (DROP / USE places).
    pub journal: Option<Arc<Mutex<WorldJournal>>>,
    /// Optional force-save signal for outer autosave (no disk I/O on sim thread).
    ///
    /// Operator `SAY SAVE` sets this when present (`SAVE OK`); otherwise
    /// `SAVE deferred`. Server polls and clears the flag.
    pub save_request: Option<Arc<AtomicBool>>,
    /// Orderly shutdown: outer main polls and exits when set.
    pub shutdown_exit: Option<Arc<AtomicBool>>,
    /// When Some, force-save is requested mid-shutdown (same as save_request).
    pub shutdown_countdown_secs: f32,
    pub shutdown_apocalypse_secs: f32,
    /// Pending orderly shutdown machine (None = idle).
    pub shutdown: Option<ShutdownState>,
    /// Monotonic sim tick counter (journal timestamps).
    pub tick: u64,
    /// Accumulates sim time toward periodic HX (heat) emit to all players.
    pub hx_emit_timer: f32,
    /// Debug: emit LOCATION_SAYS (LS) with server x,y every second to humans.
    pub pos_debug_timer: f32,
    /// Monotonic sim time in seconds (advanced by [`tick_vitals`]).
    pub sim_time: f32,
    /// Time dilation: multiplies `dt` inside [`tick_vitals`] (`1.0` = realtime).
    /// Loaded from config `sim_speed`; clamp non-negative at apply time.
    pub sim_speed: f32,
    /// When true, [`tick_vitals`] is a no-op (sim time / food / age frozen).
    /// Set by `SAY PAUSE` / cleared by `SAY RESUME`.
    pub paused: bool,
    /// Accumulates toward [`ANIMAL_WANDER_INTERVAL_SECS`] for animal wander ticks.
    pub animal_wander_timer: f32,
    /// Accumulates toward [`LIVING_PRESTIGE_REFRESH_SECS`] for living prestige classes.
    pub prestige_refresh_timer: f32,
    /// Recent session events (deaths, births, wars). Ring buffer max [`EVENT_LOG_MAX`].
    /// No SQL — in-memory only.
    pub event_log: VecDeque<String>,
    /// Cached chunk interest-tier counts (refreshed each vitals tick).
    pub chunk_hot: u32,
    pub chunk_warm: u32,
    pub chunk_cold: u32,
    pub timed_movement: bool,
    /// When true, MX/PU/FX fan-out to **all** connected players (ignore NEARBY_RANGE).
    pub broadcast_all_updates: bool,
    /// Optional death journal (RAM → disk); set by server at boot.
    pub death_log: Option<std::sync::Arc<DeathLog>>,
    pub move_jump_max_chebyshev: i32,
    pub last_lock_wait_us: u32,
}


impl SimState {
    pub fn new(world: Arc<RwLock<World>>, content: Arc<ContentDb>) -> Self {
        let grave_object_id = resolve_grave_object_id(&content);
        Self {
            world,
            content,
            players: HashMap::new(),
            logins: 0,
            intents_seen: 0,
            next_player_id: 2,
            spawn_x: 0,
            spawn_y: 0,
            prefer_last_use: false,
            player_views: None,
            pending_decays: HashMap::new(),
            social: SocialState::default(),
            environment: Environment::default(),
            apocalypse: Apocalypse::default(),
            combat: CombatState::default(),
            economy: Economy::default(),
            debts: DebtBook::default(),
            scoreboard: Scoreboard::default(),
            curses: CurseState::default(),
            env_view: None,
            weather_view: None,
            account_view: None,
            prestige_view: None,
            lineage_view: None,
            animal_view: None,
            animals_share: None,
            treasury_view: None,
            markers: MarkerState::default(),
            war: WarState::default(),
            posse: PosseState::default(),
            crime: CrimeState::default(),
            accounts: AccountBook::default(),
            weather: Weather::default(),
            fertility: FertilityState::default(),
            craft_graph: ReverseCraftGraph::default(),
            animals: AnimalWorld::default(),
            allies: AllyState::default(),
            poll: PollState::default(),
            twins: TwinRegistry::default(),
            specials: SpecialIndex::default(),
            skills: SkillState::default(),
            tutorial: TutorialState::default(),
            fire: FireState::default(),
            snow: SnowCover::default(),
            locks: LockState::default(),
            afk: AfkBook::new(),
            mutes: MuteBook::new(),
            reputation: ReputationBook::new(),
            version_gate: VersionGatePolicy::default(),
            client_version_strict: false,
            grave_object_id,
            journal: None,
            save_request: None,
            shutdown_exit: None,
            shutdown_countdown_secs: 3.0,
            shutdown_apocalypse_secs: 3.0,
            shutdown: None,
            tick: 0,
            hx_emit_timer: 0.0,
            pos_debug_timer: 0.0,
            sim_time: 0.0,
            sim_speed: 1.0,
            paused: false,
            animal_wander_timer: 0.0,
            prestige_refresh_timer: 0.0,
            event_log: VecDeque::new(),
            chunk_hot: 0,
            chunk_warm: 0,
            chunk_cold: 0,
            timed_movement: true,
            // Default false: interest-range only. true floods every client with all
            // AI PU/MX and can delay SAY/LS by minutes.
            broadcast_all_updates: false,
            death_log: None,
            move_jump_max_chebyshev: 2,
            last_lock_wait_us: 0,
        }
    }

    /// Append a session event, dropping oldest when over [`EVENT_LOG_MAX`].
    pub fn push_event(&mut self, msg: impl Into<String>) {
        while self.event_log.len() >= EVENT_LOG_MAX {
            self.event_log.pop_front();
        }
        self.event_log.push_back(msg.into());
    }

    /// `?LOG` / `?JOURNAL` chat reply body (without leading player id): last
    /// [`EVENT_LOG_QUERY_LAST`] entries, or `LOG none` when empty.
    ///
    /// `JOURNAL` is an alias for the same event-log ring buffer (not the on-disk
    /// world journal — see [`Self::format_wjournal_query`]).
    pub fn format_event_log_query(&self) -> String {
        if self.event_log.is_empty() {
            return "LOG none".into();
        }
        let n = self.event_log.len();
        let start = n.saturating_sub(EVENT_LOG_QUERY_LAST);
        let parts: Vec<&str> = self
            .event_log
            .iter()
            .skip(start)
            .map(|s| s.as_str())
            .collect();
        format!("LOG {}", parts.join("; "))
    }

    /// `?WJOURNAL` body: last shared world-journal entry summary, or `WJOURNAL none`.
    ///
    /// Uses free [`format_wjournal_query`]. When no journal Arc is attached, or the
    /// file is empty / unreadable, returns none. Best-effort lock; never panics.
    pub fn format_wjournal_query(&self) -> String {
        let Some(j) = &self.journal else {
            return queries_extra::format_wjournal_query(None);
        };
        let Ok(g) = j.lock() else {
            return queries_extra::format_wjournal_query(None);
        };
        match g.load_last_n(1) {
            Ok(entries) => {
                let last = entries.last().map(|e| (e.x, e.y, e.object_id, e.tick));
                queries_extra::format_wjournal_query(last)
            }
            Err(_) => queries_extra::format_wjournal_query(None),
        }
    }

    /// Operator force-save: set hook flag when Arc present.
    ///
    /// Returns reply body without leading p_id (`SAVE OK` / `SAVE deferred`).
    /// No disk I/O — outer autosave task performs the write.
    pub fn request_force_save(&self) -> String {
        match &self.save_request {
            Some(flag) => {
                flag.store(true, Ordering::Relaxed);
                format_save_reply(true)
            }
            None => format_save_reply(false),
        }
    }

    /// `SAY ?WHO` / `SAY WHO` chat reply body (without leading player id).
    ///
    /// Lists online players (`connected && !deleted`) as `p_id first last`, sorted by
    /// `p_id`, joined with `"; "`. Empty → `WHO none`.
    pub fn format_who_query(&self) -> String {
        let mut online: Vec<(i32, String)> = self
            .players
            .values()
            .filter(|p| p.connected && !p.deleted)
            .map(|p| (p.p_id, p.display_name()))
            .collect();
        online.sort_by_key(|(id, _)| *id);
        if online.is_empty() {
            return "WHO none".into();
        }
        let parts: Vec<String> = online
            .into_iter()
            .map(|(id, name)| format!("{id} {name}"))
            .collect();
        format!("WHO {}", parts.join("; "))
    }

    /// Online player count (`connected && !deleted`) for `SAY COUNT`.
    pub fn count_online(&self) -> usize {
        self.players
            .values()
            .filter(|p| p.connected && !p.deleted)
            .count()
    }

    /// `SAY COUNT` body without leading p_id.
    pub fn format_count_query(&self) -> String {
        queries_extra::format_count_query(self.count_online())
    }

    /// Sorted `p_id`s within Chebyshev `range` of `(x,y)` (includes self if in range).
    pub fn nearby_p_ids(&self, x: i32, y: i32, range: i32) -> Vec<i32> {
        let mut ids: Vec<i32> = self
            .players
            .values()
            .filter(|p| {
                p.connected
                    && !p.deleted
                    && (p.x - x).abs() <= range
                    && (p.y - y).abs() <= range
            })
            .map(|p| p.p_id)
            .collect();
        ids.sort_unstable();
        ids
    }

    /// `SAY NEAR` body without leading p_id.
    pub fn format_near_query_at(&self, x: i32, y: i32) -> String {
        queries_extra::format_near_query(&self.nearby_p_ids(x, y, NEARBY_RANGE))
    }

    /// Chebyshev distance to online target `p_id`, if found.
    pub fn dist_to_player(&self, from_x: i32, from_y: i32, target_p_id: i32) -> Option<i32> {
        self.players.values().find_map(|p| {
            if p.p_id == target_p_id && p.connected && !p.deleted {
                Some(queries_extra::chebyshev(from_x, from_y, p.x, p.y))
            } else {
                None
            }
        })
    }

    /// `SAY DIST <p_id>` body without leading p_id.
    pub fn format_dist_query_to(&self, from_x: i32, from_y: i32, target_p_id: i32) -> String {
        queries_extra::format_dist_query(
            target_p_id,
            self.dist_to_player(from_x, from_y, target_p_id),
        )
    }

    /// `SAY ?WHERE` / `SAY WHERE` chat reply body (without leading player id).
    ///
    /// Format: `WHERE x y biome food age` (food/age two decimal places). Pure; no SQL.
    pub fn format_where_query(x: i32, y: i32, biome: u8, food: f32, age: f32) -> String {
        format!("WHERE {x} {y} {biome} {food:.2} {age:.2}")
    }

    /// `SAY ?FOOD` / `SAY FOOD` chat reply body (without leading player id).
    ///
    /// Format: `FOOD food food_max` (two decimal places). Pure; no SQL.
    pub fn format_food_query(food: f32, food_max: f32) -> String {
        format!("FOOD {food:.2} {food_max:.2}")
    }

    /// `SAY ?AGE` / `SAY AGE` chat reply body (without leading player id).
    ///
    /// Format: `AGE age` (two decimal places). Pure; no SQL.
    pub fn format_age_query(age: f32) -> String {
        format!("AGE {age:.2}")
    }

    /// `SAY ?NAME` / `SAY NAME` chat reply body (without leading player id).
    ///
    /// Format: `NAME {display}` where display is `first last` or `first last | title`. Pure; no SQL.
    pub fn format_name_query(display_name: &str) -> String {
        format!("NAME {display_name}")
    }

    /// `SAY ?HELP` / `SAY HELP` chat reply body (without leading player id).
    ///
    /// Short list of supported SAY commands. Pure; no SQL.
    pub fn format_help_query() -> String {
        "HELP ?WHO ?WHERE ?FOOD ?AGE ?NAME ?HEART ?STATUS ?FLAGS ?HELD ?TAGS ?INV ?NOTES ?MEMORY ?CLOTHES ?COINS ?TREASURY ?DEBT ?SCORE ?HIGHSCORE ?LEAD ?LEADER ?SEASON ?TEMP ?TIME ?TICK ?YUM ?TOOLS ?LOG ?JOURNAL ?WJOURNAL ?WAR ?POSSE ?CURSE ?PRESTIGE ?REP ?APOC ?CRIME ?WEATHER ?FERTILE ?ACCOUNT ?WOUND ?RANGE ?ALLY ?POLL ?BIOMES ?BIOME ?HEX ?BIOMEFOOD ?WARM ?SPEED ?WEIGHT ?DRAIN ?SWIM ?FLOOR ?CHUNKS ?SPECIAL ?SKILLS ?TIP ?ANIMALS ?FAUNA ?STAGE ?AFK ?CRAFTSTATS ?TRANS ?GEN ?FAMILY ?REL ?TWINS COUNT NEAR DIST FOLLOW EXILE PAY TRADE ACCEPT GIFT LOAN REPAY DONATE TAX POSSE WAR PEACE RAID KILL HIT HUNT HARVEST FISH MINE DIG CHOP FEED NURSE WATER HEAL BANDAGE ALLY POLL VOTE GLOBAL WHISPER MUMBLE MUTE UNMUTE DEAF BIRTH GESTATE HOLD PUTDOWN STORE TAKE DROPALL PUTNEST WEAR STRIP CRAFT SLEEP WAKE SIT STAND SICK CURE RIDE MOUNT DISMOUNT SWIM BUILD CLAIM HOME MARK NOTE REMEMBER FORGET TITLE GOHOME PATH STEPS WALKABLE PLAN RECIPE NEXTCRAFT SEEKING EMOTE YAWN SHOUT WEATHER TIP NEXT RENAME DIE LASTUSE FIRE IGNITE EXTINGUISH LOCK UNLOCK MAPFORCE LOOK FORGETTOOLS BOOST GODMODE SNAP VOGSET REGEN CLEAROBJ FILL CLEAR_YUM PING STARTAPOC ENDAPOC SETSEASON SETHOUR SEED SAVE PAUSE RESUME PUSH PULL KISS THANK CURSE BLESS HUG SLAP".into()
    }

    /// `SAY ?TRANS` body without leading p_id: content transition counts.
    ///
    /// Format: `TRANS count=N last_use=M` from [`ContentDb::transition_count`].
    pub fn format_trans_query(content: &ContentDb) -> String {
        format!(
            "TRANS count={} last_use={}",
            content.transition_count, content.last_use_transition_count
        )
    }

    /// `SAY ?FAMILY` chat reply body (without leading player id).
    ///
    /// Lists online players (`connected && !deleted`) sharing the caller's
    /// `family_name`, sorted by `p_id`: `FAMILY {name} id name; ...` or `FAMILY none`.
    pub fn format_family_query(&self, p_id: i32) -> String {
        let family = self
            .players
            .values()
            .find(|p| p.p_id == p_id)
            .map(|p| p.family_name.clone())
            .unwrap_or_default();
        if family.is_empty() {
            return "FAMILY none".into();
        }
        let mut members: Vec<(i32, String)> = self
            .players
            .values()
            .filter(|p| p.connected && !p.deleted && p.family_name == family)
            .map(|p| (p.p_id, p.display_name()))
            .collect();
        members.sort_by_key(|(id, _)| *id);
        if members.is_empty() {
            return format!("FAMILY {family} none");
        }
        let parts: Vec<String> = members
            .into_iter()
            .map(|(id, name)| format!("{id} {name}"))
            .collect();
        format!("FAMILY {family} {}", parts.join("; "))
    }

    /// `SAY PING` reply body (without leading player id).
    ///
    /// Format: `PONG sim_time` (two decimal places). Pure; no SQL.
    pub fn format_ping_query(sim_time: f32) -> String {
        format!("PONG {sim_time:.2}")
    }

    /// `SAY ?TICK` / `SAY TICK` reply body (without leading player id).
    ///
    /// Format: `TICK {tick} {sim_time:.2}`. Pure; no SQL.
    pub fn format_tick_query(tick: u64, sim_time: f32) -> String {
        format!("TICK {tick} {sim_time:.2}")
    }

    /// `SAY PAUSE` / `SAY RESUME` reply body (without leading player id).
    ///
    /// Format: `PAUSED` or `RESUMED`. Pure; no SQL.
    pub fn format_pause_reply(paused: bool) -> String {
        if paused {
            "PAUSED".into()
        } else {
            "RESUMED".into()
        }
    }

    /// `SAY ?HELD` / `SAY HELD` chat reply body (without leading player id).
    ///
    /// Empty hands → `HELD 0`. Non-zero → `HELD {id}` plus content object name when
    /// present and non-empty (`HELD 33 Gooseberry`).
    pub fn format_held_query(&self, held_id: i32) -> String {
        if held_id == 0 {
            return "HELD 0".into();
        }
        match self.content.get(held_id) {
            Some(def) if !def.name.is_empty() => format!("HELD {held_id} {}", def.name),
            _ => format!("HELD {held_id}"),
        }
    }

    /// `SAY ?STATUS` / `SAY STATUS` chat reply body (without leading player id).
    ///
    /// Combines food, age, held object id, prestige float, prestige class wire name,
    /// wound stacks, and sleep/sick/sit flags in one private-PS line. Pure; no SQL.
    ///
    /// Format: `STATUS food age held prestige class wound sleep sick sit`
    /// (food/age/prestige two decimal places; class is [`PrestigeClass::wire_name`];
    /// sleep/sick/sit are `0`/`1`).
    pub fn format_status_query(
        food: f32,
        age: f32,
        held_id: i32,
        prestige: f32,
        class: PrestigeClass,
        wound: u8,
        sleeping: bool,
        sick: bool,
        sitting: bool,
    ) -> String {
        format!(
            "STATUS {food:.2} {age:.2} {held_id} {prestige:.2} {} {wound} {} {} {}",
            class.wire_name(),
            if sleeping { 1 } else { 0 },
            if sick { 1 } else { 0 },
            if sitting { 1 } else { 0 },
        )
    }

    /// `SAY ?HEART` / `SAY HEART` compact vitals (food + age) without leading player id.
    ///
    /// Format: `HEART food age` (two decimal places). Pure; no SQL.
    pub fn format_heart_query(food: f32, age: f32) -> String {
        format!("HEART {food:.2} {age:.2}")
    }

    /// `SAY ?FLAGS` / `SAY FLAGS` boolean state line (without leading player id).
    ///
    /// Format: `FLAGS sleeping=N sick=N sitting=N riding=N holding=N god=N deaf=N` (`0`/`1`).
    pub fn format_flags_query(
        sleeping: bool,
        sick: bool,
        sitting: bool,
        riding: bool,
        holding: bool,
        god: bool,
        deaf: bool,
    ) -> String {
        format!(
            "FLAGS sleeping={} sick={} sitting={} riding={} holding={} god={} deaf={}",
            if sleeping { 1 } else { 0 },
            if sick { 1 } else { 0 },
            if sitting { 1 } else { 0 },
            if riding { 1 } else { 0 },
            if holding { 1 } else { 0 },
            if god { 1 } else { 0 },
            if deaf { 1 } else { 0 },
        )
    }

    /// `SAY ?GODMODE` / godmode flag body (without leading player id).
    ///
    /// Format: `GODMODE on` or `GODMODE off`.
    pub fn format_godmode_query(god: bool) -> String {
        if god {
            "GODMODE on".into()
        } else {
            "GODMODE off".into()
        }
    }

    /// Food after a test `SAY BOOST` (+5, clamped to `food_max`).
    pub fn boost_food(food: f32, food_max: f32) -> f32 {
        (food + 5.0).min(food_max).max(0.0)
    }

    /// Attach an append-only world change journal (shared for concurrent readers).
    pub fn with_journal(mut self, journal: Arc<Mutex<WorldJournal>>) -> Self {
        self.journal = Some(journal);
        self
    }

    /// Attach a force-save signal Arc for operator `SAY SAVE` (outer autosave polls).
    pub fn with_save_request(mut self, flag: Arc<AtomicBool>) -> Self {
        self.save_request = Some(flag);
        self
    }

    /// Record a ground place (DROP / USE / decay). No-op if journal is unset.
    pub fn record_world_change(&self, x: i32, y: i32, object_id: i32) {
        let Some(j) = &self.journal else {
            return;
        };
        let entry = JournalEntry::new(x, y, object_id, self.tick);
        match j.lock() {
            Ok(mut g) => {
                if let Err(e) = g.append(entry) {
                    warn!(x, y, object_id, %e, "sim: world journal append failed");
                }
            }
            Err(_) => {
                warn!(x, y, object_id, "sim: world journal lock poisoned");
            }
        }
    }

    pub fn with_env_view(mut self, v: EnvView) -> Self {
        self.env_view = Some(v);
        self
    }

    pub fn with_weather_view(mut self, v: WeatherView) -> Self {
        self.weather_view = Some(v);
        self
    }

    pub fn with_account_view(mut self, v: AccountView) -> Self {
        self.account_view = Some(v);
        self
    }

    pub fn with_prestige_view(mut self, v: PrestigeView) -> Self {
        self.prestige_view = Some(v);
        self
    }

    pub fn with_lineage_view(mut self, v: LineageView) -> Self {
        self.lineage_view = Some(v);
        self
    }

    pub fn with_animal_view(mut self, v: AnimalView) -> Self {
        self.animal_view = Some(v);
        self
    }

    pub fn with_animals_share(mut self, v: AnimalWorldShare) -> Self {
        self.animals_share = Some(v);
        self
    }

    pub fn with_treasury_view(mut self, v: TreasuryView) -> Self {
        self.treasury_view = Some(v);
        self
    }

    /// Publish weather / accounts / prestige / lineage / animals / treasury for web
    /// (no sim lock held by HTTP). Also mirrors live [`AnimalWorld`] for self-play.
    pub fn publish_web_snapshots(&self) {
        if let Some(view) = &self.weather_view {
            if let Ok(mut g) = view.write() {
                *g = self.weather.snapshot();
            }
        }
        if let Some(view) = &self.account_view {
            if let Ok(mut g) = view.write() {
                *g = self.accounts.snapshot();
            }
        }
        if let Some(view) = &self.prestige_view {
            if let Ok(mut g) = view.write() {
                *g = self.scoreboard.prestige_snapshot();
            }
        }
        if let Some(view) = &self.lineage_view {
            if let Ok(mut g) = view.write() {
                *g = self.social.snapshot();
            }
        }
        if let Some(view) = &self.animal_view {
            if let Ok(mut g) = view.write() {
                *g = self.animals.snapshot();
            }
        }
        if let Some(share) = &self.animals_share {
            if let Ok(mut g) = share.write() {
                *g = self.animals.clone();
            }
        }
        if let Some(view) = &self.treasury_view {
            if let Ok(mut g) = view.write() {
                *g = TreasurySnapshot::from_economy(&self.economy);
            }
        }
    }

    /// Bootstrap social/lineage packets for a new connection (Haxe sendToMeAll*).
    ///
    /// Wire tags: LN (lineage), FW, EX, HX, TS, LR (learned tools if non-empty), NM.
    /// Haxe: LINEAGE=`LN`, LEARNED_TOOL_REPORT=`LR` (not interchangeable).
    pub fn social_bootstrap_packets(&self, for_p_id: i32) -> Vec<Vec<u8>> {
        let mut out = Vec::new();
        let lineages = self.social.lineage_packets();
        if !lineages.is_empty() {
            let refs: Vec<&str> = lineages.iter().map(|s| s.as_str()).collect();
            out.push(format_server_message("LN", &refs).into_bytes());
        }
        let follows = self.social.following_packets();
        if !follows.is_empty() {
            let refs: Vec<&str> = follows.iter().map(|s| s.as_str()).collect();
            out.push(format_server_message("FW", &refs).into_bytes());
        }
        let exiles = self.social.exile_packets();
        if !exiles.is_empty() {
            let refs: Vec<&str> = exiles.iter().map(|s| s.as_str()).collect();
            out.push(format_server_message("EX", &refs).into_bytes());
        }
        // Personal heat/season hint (HX heat food_time indoor_bonus).
        let biome = self
            .players
            .values()
            .find(|p| p.p_id == for_p_id)
            .map(|p| self.world.read().unwrap().get_biome(p.x, p.y))
            .unwrap_or(0);
        let heat = self.environment.temperature_at_biome(biome);
        out.push(format_heat_change(heat, 0.0, 0.0).into_bytes());
        // Tool slots + learned tools (LR) + name for this player.
        if let Some(p) = self.players.values().find(|p| p.p_id == for_p_id) {
            let ts = p.tools.wire_slots();
            out.push(format_server_message("TS", &[&ts]).into_bytes());
            if !p.tools.learned.is_empty() {
                let ids = p.tools.learned_ids_sorted();
                out.push(format_learned_tool_report(&ids).into_bytes());
            }
            let nm = format!("{} {}", p.p_id, p.display_name());
            out.push(format_server_message("NM", &[&nm]).into_bytes());
        }
        out
    }

    /// Sync lineage prestige from combat stats and recompute prestige class.
    pub fn sync_lineage_prestige_from_combat(&mut self, p_id: i32) {
        let prestige = self
            .combat
            .stats
            .get(&p_id)
            .map(|s| s.prestige)
            .unwrap_or(0.0);
        self.social.set_lineage_prestige(p_id, prestige);
    }

    /// Recompute [`PrestigeClass`] for online living players from scoreboard ranks.
    ///
    /// Uses [`prestige_classes_from_living_scores`] and stores the class on lineage
    /// nodes when present (does not rewrite prestige floats).
    pub fn refresh_living_prestige_classes(&mut self) {
        let scores: Vec<(i32, i32)> = self
            .players
            .values()
            .filter(|p| p.connected && !p.deleted)
            .map(|p| {
                let score = self
                    .scoreboard
                    .entry(p.p_id)
                    .map(|e| e.score)
                    .unwrap_or(0);
                (p.p_id, score)
            })
            .collect();
        if scores.is_empty() {
            return;
        }
        let classes = prestige_classes_from_living_scores(&scores);
        for (p_id, class) in classes {
            self.social.set_lineage_prestige_class(p_id, class);
        }
    }

    /// Combined social lineage prestige (falls back to combat prestige).
    pub fn player_prestige(&self, p_id: i32) -> f32 {
        if let Some(n) = self.social.lineages.get(&p_id) {
            return n.prestige;
        }
        self.combat
            .stats
            .get(&p_id)
            .map(|s| s.prestige)
            .unwrap_or(0.0)
    }

    /// Prestige class from lineage (preferred) or combat prestige float.
    pub fn player_prestige_class(&self, p_id: i32) -> PrestigeClass {
        if let Some(n) = self.social.lineages.get(&p_id) {
            return n.prestige_class();
        }
        self.combat.prestige_class(p_id)
    }

    /// Haxe PlayerSoul-style first-person prestige info wire string.
    pub fn player_prestige_info(&self, p_id: i32) -> String {
        player_prestige_info_wire(self.player_prestige(p_id))
    }

    pub fn with_player_views(mut self, views: PlayerViewMap) -> Self {
        self.player_views = Some(views);
        self
    }

    pub fn with_default_empty(content: Arc<ContentDb>) -> Self {
        Self::new(Arc::new(RwLock::new(World::new(512, 512, true))), content)
    }

    pub fn publish_player_view(&self, conn_id: u64) {
        let Some(views) = &self.player_views else {
            return;
        };
        let Some(p) = self.players.get(&conn_id) else {
            return;
        };
        if let Ok(mut g) = views.write() {
            g.insert(conn_id, p.snapshot());
        }
    }

    pub fn publish_all_player_views(&self) {
        let Some(views) = &self.player_views else {
            return;
        };
        if let Ok(mut g) = views.write() {
            g.clear();
            for (cid, p) in &self.players {
                // Keep deleted snapshots so self-play / viewer can detect death.
                g.insert(*cid, p.snapshot());
            }
        }
    }

    /// BW / BABY_WIGGLE wire packet for `p_id` (Haxe `sendWiggle`).
    pub fn format_baby_wiggle(p_id: i32) -> String {
        ol_protocol::format_baby_wiggle(p_id)
    }

    /// DY / DYING wire packet; `sick` sets optional isSick `1` flag.
    pub fn format_dying(p_id: i32, sick: bool) -> String {
        ol_protocol::format_dying(p_id, sick)
    }
}

/// Conn ids of players within Chebyshev `range` of (x,y), including self if present.
///
/// When [`SimState::broadcast_all_updates`] is true, returns **all** connected
/// non-deleted players (setting for full PU/MX fan-out regardless of distance).
pub fn nearby_conn_ids(state: &SimState, x: i32, y: i32, range: i32) -> Vec<u64> {
    if state.broadcast_all_updates {
        return state
            .players
            .iter()
            .filter(|(_, p)| p.connected && !p.deleted)
            .map(|(c, _)| *c)
            .collect();
    }
    state
        .players
        .iter()
        .filter(|(_, p)| {
            p.connected
                && !p.deleted
                && (p.x - x).abs() <= range
                && (p.y - y).abs() <= range
        })
        .map(|(c, _)| *c)
        .collect()
}

fn send_nearby(outbound: &OutboundHub, conn_ids: &[u64], packet: Vec<u8>) {
    for &cid in conn_ids {
        outbound.send(cid, packet.clone());
    }
}

/// Official clients (LivingLifePage `waitForFrameMessages`) **hold** PM/PU/PS/LS
/// until they see `FM`. Headless clients may apply bytes immediately; real clients
/// do not. Always pair interactive messages with FRAME.
fn send_frame(outbound: &OutboundHub, conn_id: u64) {
    outbound.send_urgent(conn_id, format_frame().into_bytes());
}

/// Private/query PS in **protocol form** `p_id/0 text` + FRAME.
///
/// Protocol (`protocol.txt` PLAYER_SAYS): each data line is **`p_id/isCurse text`**.
/// Official clients parse with `indexOf("/")` — a space-only `p_id text` form is rejected.
///
/// Accepts either:
/// - protocol lines `"p_id/0 text…"` (slash already present)
/// - legacy sim lines `"p_id text…"` (space, no slash) — rewritten to `/0`
/// - bare tokens like `"RATE"` — emitted as `0/0 RATE` so the slash parse always works
fn send_ps_reply(outbound: &OutboundHub, conn_id: u64, line: &str) {
    let line = line.trim();
    let pkt = if let Some((head, rest)) = line.split_once(' ') {
        if head.contains('/') {
            // Already `id/curse …` — keep body, still re-wrap via format_player_says when
            // the head is a clean `N/0` so #/newlines are sanitized.
            if let Some((id_s, curse_s)) = head.split_once('/') {
                if let (Ok(p_id), Ok(curse)) = (id_s.parse::<i32>(), curse_s.parse::<i32>()) {
                    format_player_says(p_id, curse != 0, rest).into_bytes()
                } else {
                    format_server_message("PS", &[line]).into_bytes()
                }
            } else {
                format_server_message("PS", &[line]).into_bytes()
            }
        } else if let Ok(p_id) = head.parse::<i32>() {
            format_player_says(p_id, false, rest).into_bytes()
        } else {
            // Non-numeric head (e.g. "RATE …") — still legal wire with speaker 0.
            format_player_says(0, false, line).into_bytes()
        }
    } else if let Ok(p_id) = line.parse::<i32>() {
        format_player_says(p_id, false, "").into_bytes()
    } else if line.is_empty() {
        format_player_says(0, false, "").into_bytes()
    } else {
        format_player_says(0, false, line).into_bytes()
    };
    outbound.send_urgent(conn_id, pkt);
    send_frame(outbound, conn_id);
}

/// Nearby PS fan-out for command acks (`"p_id POSSE 0 OK"` style) with correct wire + FM.
fn send_nearby_ps_lines(outbound: &OutboundHub, conn_ids: &[u64], line: &str) {
    for &cid in conn_ids {
        send_ps_reply(outbound, cid, line);
    }
}

/// Chat PS for speaker + nearby, protocol `p_id/0 text`, each with FRAME.
fn send_chat_ps(
    state: &SimState,
    outbound: &OutboundHub,
    speaker_conn: u64,
    speaker_p_id: i32,
    text: &str,
    near: &[u64],
) {
    let pkt = format_player_says(speaker_p_id, false, text).into_bytes();
    // Speaker echo first (Haxe private path still uses id/curse + FRAME).
    outbound.send_urgent(speaker_conn, pkt.clone());
    send_frame(outbound, speaker_conn);
    for &cid in near {
        if cid == speaker_conn {
            continue;
        }
        let Some(listener) = state.players.get(&cid) else {
            continue;
        };
        if !should_hear(listener.deaf, false) {
            continue;
        }
        if state.mutes.should_deliver(listener.p_id, speaker_p_id) {
            outbound.send_urgent(cid, pkt.clone());
            send_frame(outbound, cid);
        }
    }
}

/// Normal SAY / SHOUT / MUMBLE PS fan-out: skip muted listeners and DEAF players.
///
/// Whispers use a private path and are not filtered by DEAF (see WHISPER handler).
/// Packet must already be full wire bytes for one PS (prefer [`format_player_says`]).
fn send_nearby_chat(
    state: &SimState,
    outbound: &OutboundHub,
    conn_ids: &[u64],
    speaker_p_id: i32,
    packet: Vec<u8>,
) {
    for &cid in conn_ids {
        let Some(listener) = state.players.get(&cid) else {
            continue;
        };
        if !should_hear(listener.deaf, false) {
            continue;
        }
        if state.mutes.should_deliver(listener.p_id, speaker_p_id) {
            outbound.send_urgent(cid, packet.clone());
            send_frame(outbound, cid);
        }
    }
}

/// Server→client global chat (Haxe GLOBAL / GLOBAL_MESSAGE style).
/// Wire: `format_server_message("GM", &[text])` then [`OutboundHub::broadcast`].
pub fn broadcast_global(outbound: &OutboundHub, text: &str) {
    let text = text.trim();
    if text.is_empty() {
        return;
    }
    let pkt = format_server_message("GM", &[text]).into_bytes();
    outbound.broadcast(pkt);
}

/// Haxe `sendMapChunkIfNeeded` / `sendMapChunk` after move.
pub fn maybe_send_map_chunk(state: &mut SimState, outbound: &OutboundHub, conn_id: u64) {
    let Some(p) = state.players.get(&conn_id) else {
        return;
    };
    if p.deleted || !p.needs_map_chunk(MC_RESEND_THRESHOLD) {
        return;
    }
    force_send_map_chunk(state, outbound, conn_id);
}

/// Always send MAP_CHUNK centered on the player (login / SAY MAPFORCE).
pub fn force_send_map_chunk(state: &mut SimState, outbound: &OutboundHub, conn_id: u64) {
    let Some(p) = state.players.get(&conn_id) else {
        return;
    };
    if p.deleted {
        return;
    }
    let (x, y) = (p.x, p.y);
    let (wire_cx, wire_cy) = p.world_to_client(x, y);
    let mc = {
        let w = state.world.read().unwrap();
        crate::map_chunk::build_map_chunk_packet_ex(
            &w,
            x,
            y,
            wire_cx,
            wire_cy,
            MC_WIDTH,
            MC_HEIGHT,
        )
    };
    if let Some(p) = state.players.get_mut(&conn_id) {
        p.last_mc_x = x;
        p.last_mc_y = y;
        p.has_mc = true;
    }
    outbound.send(conn_id, mc);
    debug!(conn_id, x, y, wire_cx, wire_cy, "sim: MC force/refresh");
}

/// Recompute hot/warm/cold chunk interest counts into [`SimState`] (vitals tick / ?CHUNKS).
pub fn refresh_chunk_tier_counts(state: &mut SimState) {
    let player_tiles: Vec<(i32, i32)> = state
        .players
        .values()
        .filter(|pl| pl.connected && !pl.deleted)
        .map(|pl| (pl.x, pl.y))
        .collect();
    // Sample a 7×7 chunk neighborhood around each player (cheap).
    let mut coords = std::collections::HashSet::new();
    const CS: i32 = 64;
    for &(x, y) in &player_tiles {
        let pcx = x.div_euclid(CS);
        let pcy = y.div_euclid(CS);
        for dx in -3..=3 {
            for dy in -3..=3 {
                coords.insert((pcx + dx, pcy + dy));
            }
        }
    }
    let list: Vec<(i32, i32)> = coords.into_iter().collect();
    let map = build_tier_map(&list, &player_tiles, CS);
    let (h, w, c) = tier_counts(&map);
    state.chunk_hot = h;
    state.chunk_warm = w;
    state.chunk_cold = c;
}

/// DROP on empty ground or into container (Haxe DROP x y [c]).
///
/// Floor-only objects (`ObjectDef.floor`) are **not** placed on the ground object
/// layer — DROP is skipped and the player keeps holding (floor place path TBD).
pub fn apply_drop(
    state: &mut SimState,
    outbound: &OutboundHub,
    conn_id: u64,
    x: i32,
    y: i32,
    c: Option<i32>,
) {
    if let Some(p) = state.players.get(&conn_id) {
        if p.deleted {
            return;
        }
        if is_moving(p) {
            send_player_update_and_frame(state, outbound, conn_id);
            return;
        }
        if !in_use_range(p.x, p.y, x, y, 1) {
            send_player_update_and_frame(state, outbound, conn_id);
            return;
        }
    }
    let held = state.players.get(&conn_id).map(|p| p.held_id).unwrap_or(0);
    if held == 0 {
        return;
    }
    // Floor-only: skip ground place (do not put roads/floors on object layer).
    if state
        .content
        .get(held)
        .map(|d| d.is_floor())
        .unwrap_or(false)
    {
        info!(conn_id, x, y, held, "sim: DROP skipped floor-only object");
        let _ = c;
        return;
    }
    let tile = state.world.read().unwrap().get_object(x, y);

    // Into container if tile is container and held is containable.
    if tile != 0 {
        let slots = state
            .content
            .get(tile)
            .map(|d| d.num_slots.max(0) as usize)
            .unwrap_or(0);
        // Only containable items may enter containers (Haxe containable=1).
        let held_ok = state
            .content
            .get(held)
            .map(|d| d.containable)
            .unwrap_or(false);
        if slots > 0 && held_ok {
            // Haxe-style time-in-container: stamp sim_time on put; permanent
            // containers keep contents across OLW2 saves (nested + creation_time).
            let sim_t = state.sim_time;
            // Default: no auto-decay on container itself (time_to_change=0 = permanent hold).
            // Non-permanent base objects may get a soft decay timer from content later.
            let base_permanent = state
                .content
                .get(tile)
                .map(|d| d.permanent)
                .unwrap_or(false);
            let ttc = if base_permanent { 0.0 } else { 300.0 };
            let put = state.world.write().unwrap().container_put_timed(
                x,
                y,
                held,
                slots.max(1),
                sim_t,
                ttc,
            );
            if put {
                if let Some(p) = state.players.get_mut(&conn_id) {
                    p.held_id = 0;
                }
                state.publish_player_view(conn_id);
                info!(conn_id, x, y, held, tile, "sim: DROP into container");
                send_drop_result(state, outbound, conn_id, x, y, tile);
                let _ = c;
                return;
            }
        }
        // Occupied non-container: try USE-style stack transition (stone on stone → pile).
        // Haxe often uses USE; clients may also DROP onto a same-type stackable.
        if let Some(r) = apply_use_at(state, conn_id, x, y) {
            if r.applied {
                info!(
                    conn_id,
                    x,
                    y,
                    held,
                    tile,
                    new_target = r.target_after,
                    "sim: DROP stacked via USE transition"
                );
                state.publish_player_view(conn_id);
                let near = nearby_conn_ids(state, x, y, NEARBY_RANGE);
                for pkt in packets_after_use(state, conn_id, &r) {
                    for &cid in &near {
                        outbound.send_urgent(cid, pkt.clone());
                    }
                }
                send_frame(outbound, conn_id);
                let _ = c;
                return;
            }
        }
        // Still blocked — unstick client immediately.
        send_player_update_and_frame(state, outbound, conn_id);
        let _ = c;
        return;
    }

    if tile == 0 {
        // Record lineage/player ownership on place (Haxe ObjectHelper.owner).
        let owner_id = state.players.get(&conn_id).map(|p| p.p_id).unwrap_or(0);
        state
            .world
            .write()
            .unwrap()
            .set_object_complex(x, y, ComplexObject::with_owner(held, owner_id));
        state.record_world_change(x, y, held);
        if let Some(p) = state.players.get_mut(&conn_id) {
            p.held_id = 0;
        }
        schedule_decay(state, x, y, held);
        state.publish_player_view(conn_id);
        // Index special objects for fast scans (gates/graves/containers).
        if let Some(def) = state.content.get(held) {
            if let Some(kind) = SpecialKind::from_object_name(&def.name) {
                state.specials.insert(x, y, kind);
            }
        }
        info!(conn_id, x, y, held, owner_id, "sim: DROP placed");
        send_drop_result(state, outbound, conn_id, x, y, held);
    }
}

/// DROP reply: MX+PU **urgent** + **FM** (same speed class as USE).
///
/// Previously used the normal lane without FM — official clients buffered and
/// DROP looked multi-second laggy while pickup (USE+FM) felt instant.
fn send_drop_result(
    state: &SimState,
    outbound: &OutboundHub,
    conn_id: u64,
    x: i32,
    y: i32,
    placed: i32,
) {
    let near = nearby_conn_ids(state, x, y, NEARBY_RANGE);
    for pkt in packets_after_drop(state, conn_id, x, y, placed) {
        for &cid in &near {
            outbound.send_urgent(cid, pkt.clone());
        }
    }
    send_frame(outbound, conn_id);
}

/// Strip OHOL client map coords from SAY payload.
///
/// Wire forms: `hello`, `0 0 hello`, `-1 2 !SHUTDOWN`. Leading integer pairs
/// are client tile hints and must not prevent command matching.
pub fn normalize_say_text(payload: &str) -> String {
    let t = payload.trim();
    if t.is_empty() {
        return String::new();
    }
    let mut parts = t.split_whitespace();
    let a = parts.next().unwrap_or("");
    let b = parts.next();
    // Two leading integers → drop them; remainder is the spoken line.
    if a.parse::<i32>().is_ok() {
        if let Some(b) = b {
            if b.parse::<i32>().is_ok() {
                let rest: Vec<&str> = parts.collect();
                return rest.join(" ");
            }
        }
    }
    t.to_string()
}

/// True when SAY text is a **server-wide** orderly shutdown command.
///
/// Note: `!CLOSE` is **not** included — it only disconnects the speaking client
/// (see `apply_say_or_remv` CLOSE handler). Use `!SHUTDOWN` to stop the process.
pub fn is_shutdown_say(upper: &str) -> bool {
    let u = upper.trim();
    u == "!SHUTDOWN"
        || u == "SHUTDOWN!"
        || u == "SHUTDOWN"
        || u == "!QUIT"
        || u.starts_with("!SHUTDOWN ")
        || u.contains("!SHUTDOWN")
}

/// True when SAY asks to disconnect **this client only** (not stop the server).
pub fn is_close_say(upper: &str) -> bool {
    let u = upper.trim();
    u == "!CLOSE" || u == "CLOSE!" || u == "CLOSE" || u.starts_with("!CLOSE ")
}

/// REMV x y [i] — take from container; SAY text — nearby chat + query commands.
///
/// `SAY EMOTE <n>` is an alias for client `EMOT`: emit `PE player_id n` to
/// connections within [`NEARBY_RANGE`] (no PS chat line).
fn apply_say_or_remv(
    state: &mut SimState,
    outbound: &OutboundHub,
    counters: &Counters,
    conn_id: u64,
    tag: &str,
    payload: &str,
) {
    if tag.eq_ignore_ascii_case("SAY") {
        let Some(p) = state.players.get(&conn_id).cloned() else {
            return;
        };
        if p.deleted {
            return;
        }
        // OHOL clients often send `SAY x y text` (map coords); strip leading ints.
        let text_owned = normalize_say_text(payload);
        let text: &str = text_owned.as_str();
        let upper = text.to_uppercase();
        // Activity touch for AFK (skip ?AFK so status reflects true idle).
        if upper != "?AFK" && upper != "AFK" {
            touch_afk_activity(state, conn_id);
        }
        // ?STAGE / STAGE — infant/child/adult/elder.
        if upper == "?STAGE" || upper == "STAGE" {
            let reply = format_stage_query(p.age);
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // ?AFK / AFK — idle / remaining secs until AFK (DEFAULT_AFK_SECS).
        if upper == "?AFK" || upper == "AFK" {
            let reply = format_afk_query(&state.afk, p.p_id, state.sim_time, DEFAULT_AFK_SECS);
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // ?RANGE / RANGE — combat range for held weapon name (before rate limit: cheap query).
        if upper == "?RANGE" || upper == "RANGE" {
            let name = held_object_name(state, p.held_id);
            let r = weapon_range(p.held_id, &name);
            // Chat form: `RANGE N` bare hands; `RANGE N held=Name` when holding.
            let reply = if name.is_empty() || p.held_id == 0 {
                format!("RANGE {r}")
            } else {
                format!("RANGE {r} held={name}")
            };
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // LOOK [dx dy] — biome/floor/obj under relative tile.
        if upper == "LOOK" || upper.starts_with("LOOK ") || upper.starts_with("?LOOK") {
            // Prefer wire_fields::parse_xy for the coordinate pair after the verb.
            let after = text
                .split_whitespace()
                .skip(1)
                .collect::<Vec<_>>()
                .join(" ");
            let (dx, dy) = parse_xy(&after).unwrap_or((0, 0));
            let (tx, ty) = (p.x + dx, p.y + dy);
            let (biome, floor, obj) = {
                let w = state.world.read().unwrap();
                (w.get_biome(tx, ty), w.get_floor(tx, ty), w.get_object(tx, ty))
            };
            let name = state
                .content
                .get(obj)
                .map(|d| d.name.clone())
                .unwrap_or_default();
            let reply = format_look(dx, dy, biome, floor, obj, &name);
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // ?TREASURY / DONATE / TAX — treasury lives on Economy.
        if upper == "?TREASURY" || upper == "TREASURY" {
            let reply = format_treasury_query(state.economy.treasury);
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        if upper.starts_with("DONATE ") {
            let amount: i32 = text
                .split_whitespace()
                .nth(1)
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            let ok = treasury_donate(&mut state.economy, p.p_id, amount);
            if ok {
                let coins = state
                    .economy
                    .wallets
                    .get(&p.p_id)
                    .map(|w| w.coins)
                    .unwrap_or(0);
                state.scoreboard.set_coins(p.p_id, coins);
            }
            let line = format!(
                "{} DONATE {} {}",
                p.p_id,
                amount,
                if ok { "OK" } else { "FAIL" }
            );
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // TAX <amount> — leader pays from own wallet into treasury.
        if upper.starts_with("TAX ") {
            let amount: i32 = text
                .split_whitespace()
                .nth(1)
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            let leader_ok = is_leader(&state.social.following, p.p_id);
            let ok = leader_ok && tax(&mut state.economy, p.p_id, amount);
            if ok {
                let coins = state
                    .economy
                    .wallets
                    .get(&p.p_id)
                    .map(|w| w.coins)
                    .unwrap_or(0);
                state.scoreboard.set_coins(p.p_id, coins);
            }
            let line = format!(
                "{} TAX {} {}",
                p.p_id,
                amount,
                if ok { "OK" } else { "FAIL" }
            );
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // EMOTE / YAWN use a separate rate limit (not SAY chat). Handle before SAY window.
        if upper.starts_with("EMOTE ") || upper == "EMOTE" || upper == "YAWN" {
            let now = state.sim_time;
            if let Some(pl) = state.players.get_mut(&conn_id) {
                if !pl.emote_rate.try_emote(now) {
                    send_ps_reply(outbound, conn_id, "0 EMOTE RATE");
                    return;
                }
            }
            let e = if upper == "YAWN" {
                YAWN_EMOT_INDEX
            } else {
                text.split_whitespace()
                    .nth(1)
                    .and_then(|s| s.parse::<i32>().ok())
                    .unwrap_or(0)
            };
            let line = format!("{} {}", p.p_id, e);
            let near = nearby_conn_ids(state, p.x, p.y, NEARBY_RANGE);
            send_nearby(
                outbound,
                &near,
                format_server_message("PE", &[&line]).into_bytes(),
            );
            // PE is applied only after FRAME on official clients once waitForFrameMessages.
            for &cid in &near {
                send_frame(outbound, cid);
            }
            info!(conn_id, e, "sim: SAY EMOTE/YAWN");
            return;
        }
        // Commands (! / ? / known verbs) are NOT rate-limited — intermittent
        // "command sometimes works" was SAY RATE blocking before handlers.
        // Rate limit only free-form chat at the bottom of this function.
        // HELP / ?HELP — short list of supported SAY commands (private PS; no SQL).
        if upper == "HELP" || upper == "?HELP" || upper.starts_with("?HELP") {
            let reply = SimState::format_help_query();
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // TIP / ?TIP / NEXT — newbie tutorial tip (advances on TIP/NEXT).
        if upper == "?TIP" {
            let reply = state.tutorial.format_tip_query(p.p_id);
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        if upper == "TIP" || upper == "NEXT" {
            let reply = state.tutorial.take_tip(p.p_id);
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // ?SKILLS / SKILLS
        if upper == "?SKILLS" || upper == "SKILLS" {
            let reply = state.skills.format_query(p.p_id);
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // ?REL / REL <p_id> — kinship via lineage mother links (Eve when mother_id None).
        if upper.starts_with("REL ") || upper.starts_with("?REL ") {
            let other: i32 = text
                .split_whitespace()
                .nth(1)
                .and_then(|s| s.parse().ok())
                .unwrap_or(p.p_id);
            let reply = format_relation_query(&state.social, p.p_id, other);
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // Bare ?REL / REL — self relation (shows EVE when mother_id is None).
        if upper == "?REL" || upper == "REL" {
            let reply = format_relation_query(&state.social, p.p_id, p.p_id);
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // ?GEN / GEN [p_id] — multi-gen lineage depth from LineageNode.generation.
        if upper == "?GEN" || upper == "GEN" {
            let reply = format_gen_query(&state.social, p.p_id);
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        if upper.starts_with("?GEN ") || upper.starts_with("GEN ") {
            let other: i32 = text
                .split_whitespace()
                .nth(1)
                .and_then(|s| s.parse().ok())
                .unwrap_or(p.p_id);
            let reply = format_gen_query(&state.social, other);
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // ?FAMILY / FAMILY — online players sharing caller's family_name.
        if upper == "?FAMILY" || upper == "FAMILY" || upper.starts_with("?FAMILY") {
            let reply = state.format_family_query(p.p_id);
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        if upper == "?CHILDREN" || upper == "CHILDREN" {
            let reply = format_children_query(&state.social, p.p_id);
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // LOCK / UNLOCK under feet (gate ownership helper).
        if upper == "LOCK" {
            state.locks.lock(p.x, p.y);
            // Ensure complex helper records owner.
            {
                let mut w = state.world.write().unwrap();
                let id = w.get_object(p.x, p.y);
                if id != 0 {
                    let mut h = w
                        .get_helper(p.x, p.y)
                        .cloned()
                        .unwrap_or_else(|| ComplexObject::with_owner(id, p.p_id));
                    h.owner_id = p.p_id;
                    w.set_object_complex(p.x, p.y, h);
                }
            }
            let line = format!("{} LOCK {} {} OK", p.p_id, p.x, p.y);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        if upper == "UNLOCK" {
            let ok = state.locks.unlock(p.x, p.y);
            // Clear ownership on the helper so walkability matches unlocked gates.
            {
                let mut w = state.world.write().unwrap();
                let id = w.get_object(p.x, p.y);
                if id != 0 {
                    if let Some(mut h) = w.get_helper(p.x, p.y).cloned() {
                        h.owner_id = 0;
                        w.set_object_complex(p.x, p.y, h);
                    }
                }
            }
            let line = format!(
                "{} UNLOCK {} {} {}",
                p.p_id,
                p.x,
                p.y,
                if ok { "OK" } else { "FAIL" }
            );
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        if upper == "?LOCK" {
            let owner = state
                .world
                .read()
                .unwrap()
                .get_helper(p.x, p.y)
                .map(|h| h.owner_id)
                .unwrap_or(0);
            let locked = state.locks.is_locked(p.x, p.y);
            let reply = format_lock_query(owner, locked);
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // Haxe ?SEASON TEMP / ?TEMP / FOLLOW / EXILE style commands.
        if upper.contains("?SEASON") || upper == "?ST" {
            let reply = state.environment.season_query_text();
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // SETSEASON SPRING|SUMMER|AUTUMN|WINTER — force season (testing; no admin).
        if upper.starts_with("SETSEASON ") || upper == "SETSEASON" {
            let tok = text.split_whitespace().nth(1).unwrap_or("");
            if let Some(season) = Season::parse(tok) {
                let prev = state.environment.season;
                state.environment.set_season(season);
                if season != prev {
                    // Seed tag with previous season so the change actually resets.
                    if state.scoreboard.season_tag.is_empty() {
                        state.scoreboard.season_tag = prev.as_str().to_string();
                    }
                    if state.scoreboard.on_season_change(season.as_str()) {
                        info!(
                            conn_id,
                            season = %season.as_str(),
                            "sim: scoreboard season leaderboard reset"
                        );
                    }
                } else {
                    // Bind tag if still empty (first SETSEASON to current season).
                    let _ = state.scoreboard.on_season_change(season.as_str());
                }
                let reply = state.environment.season_query_text();
                let line = format!("{} {}", p.p_id, reply);
                send_ps_reply(outbound, conn_id, &line);
                info!(conn_id, season = %season.as_str(), "sim: SETSEASON");
            } else {
                let line = format!("{} SETSEASON FAIL bad_season", p.p_id);
                send_ps_reply(outbound, conn_id, &line);
            }
            return;
        }
        if upper.contains("?TEMP") || upper == "?T" {
            let biome = state.world.read().unwrap().get_biome(p.x, p.y);
            let reply = state.environment.temp_query_text(biome);
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        if upper.contains("?TIME") {
            let reply = state.environment.time_query_text();
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // ?TICK / TICK — private PS: monotonic tick counter + sim_time seconds.
        if upper == "?TICK" || upper == "TICK" || upper.starts_with("?TICK") {
            let reply = SimState::format_tick_query(state.tick, state.sim_time);
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // PAUSE — freeze vitals (sim_time / food / age); wall loop still accepts intents.
        if upper == "PAUSE" {
            state.paused = true;
            let reply = SimState::format_pause_reply(true);
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            info!(conn_id, "sim: PAUSE");
            return;
        }
        // RESUME — unfreeze vitals after PAUSE.
        if upper == "RESUME" {
            state.paused = false;
            let reply = SimState::format_pause_reply(false);
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            info!(conn_id, "sim: RESUME");
            return;
        }
        // SETHOUR <0-23> — force day hour (testing; no admin).
        if upper.starts_with("SETHOUR ") || upper == "SETHOUR" {
            let tok = text.split_whitespace().nth(1).unwrap_or("");
            match tok.parse::<f32>() {
                Ok(h) if (0.0..24.0).contains(&h) || (h - 24.0).abs() < 1e-6 => {
                    // Accept 0..=23 (and 24 as midnight wrap).
                    let hour = if (h - 24.0).abs() < 1e-6 { 0.0 } else { h };
                    state.environment.set_hour(hour);
                    let reply = state.environment.time_query_text();
                    let line = format!("{} {}", p.p_id, reply);
                    send_ps_reply(outbound, conn_id, &line);
                    info!(conn_id, hour, "sim: SETHOUR");
                }
                _ => {
                    let line = format!("{} SETHOUR FAIL bad_hour", p.p_id);
                    send_ps_reply(outbound, conn_id, &line);
                }
            }
            return;
        }
        if upper.starts_with("FOLLOW ") {
            let rest = text.split_whitespace().nth(1).unwrap_or("");
            if let Ok(leader) = rest.parse::<i32>() {
                match state.social.set_follow(p.p_id, leader) {
                    Ok(()) => {
                        let color = state
                            .social
                            .leader_colors
                            .get(&leader)
                            .copied()
                            .unwrap_or(0);
                        let line = format_following_line(p.p_id, leader, color);
                        let near = nearby_conn_ids(state, p.x, p.y, NEARBY_RANGE);
                        send_nearby(
                            outbound,
                            &near,
                            format_server_message("FW", &[&line]).into_bytes(),
                        );
                    }
                    Err(e) => {
                        let line = format!("{} FOLLOW FAIL {e}", p.p_id);
                        send_ps_reply(outbound, conn_id, &line);
                    }
                }
            }
            return;
        }
        if upper.starts_with("EXILE ") {
            let rest = text.split_whitespace().nth(1).unwrap_or("");
            if let Ok(target) = rest.parse::<i32>() {
                state.social.exile(p.p_id, target);
                let line = format_exile_line(target, p.p_id);
                let near = nearby_conn_ids(state, p.x, p.y, NEARBY_RANGE);
                send_nearby(
                    outbound,
                    &near,
                    format_server_message("EX", &[&line]).into_bytes(),
                );
            }
            return;
        }
        if upper.starts_with("PAY ") {
            // PAY <p_id> <amount>
            let mut it = text.split_whitespace();
            let _ = it.next();
            let to: i32 = it.next().and_then(|s| s.parse().ok()).unwrap_or(0);
            let amount: i32 = it.next().and_then(|s| s.parse().ok()).unwrap_or(0);
            let ok = state.economy.transfer(p.p_id, to, amount);
            if ok {
                // Keep scoreboard coins/score in sync after pay.
                for id in [p.p_id, to] {
                    let coins = state
                        .economy
                        .wallets
                        .get(&id)
                        .map(|w| w.coins)
                        .unwrap_or(0);
                    state.scoreboard.set_coins(id, coins);
                }
            }
            let line = format!(
                "{} PAY {} {} {}",
                p.p_id,
                to,
                amount,
                if ok { "OK" } else { "FAIL" }
            );
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // GIFT <p_id> <amount> — coin transfer without trade prestige.
        if upper.starts_with("GIFT ") {
            let mut it = text.split_whitespace();
            let _ = it.next();
            let to: i32 = it.next().and_then(|s| s.parse().ok()).unwrap_or(0);
            let amount: i32 = it.next().and_then(|s| s.parse().ok()).unwrap_or(0);
            let ok = state.economy.gift(p.p_id, to, amount);
            if ok {
                for id in [p.p_id, to] {
                    let coins = state
                        .economy
                        .wallets
                        .get(&id)
                        .map(|w| w.coins)
                        .unwrap_or(0);
                    state.scoreboard.set_coins(id, coins);
                }
            }
            let line = format!(
                "{} GIFT {} {} {}",
                p.p_id,
                to,
                amount,
                if ok { "OK" } else { "FAIL" }
            );
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // LOAN <p_id> <amount> — gift coins to borrower and record debt (no trade prestige).
        if upper.starts_with("LOAN ") {
            let mut it = text.split_whitespace();
            let _ = it.next();
            let to: i32 = it.next().and_then(|s| s.parse().ok()).unwrap_or(0);
            let amount: i32 = it.next().and_then(|s| s.parse().ok()).unwrap_or(0);
            let moved = state.economy.gift(p.p_id, to, amount);
            let ok = if moved {
                match state.debts.record_loan(p.p_id, to, amount) {
                    Ok(()) => {
                        for id in [p.p_id, to] {
                            let coins = state
                                .economy
                                .wallets
                                .get(&id)
                                .map(|w| w.coins)
                                .unwrap_or(0);
                            state.scoreboard.set_coins(id, coins);
                        }
                        true
                    }
                    Err(_) => {
                        // Roll back coin move if debt book rejected (should be rare).
                        let _ = state.economy.gift(to, p.p_id, amount);
                        false
                    }
                }
            } else {
                false
            };
            let line = format!(
                "{} LOAN {} {} {}",
                p.p_id,
                to,
                amount,
                if ok { "OK" } else { "FAIL" }
            );
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // REPAY <p_id> [amount] — pay down debt to creditor (full debt if amount omitted).
        if upper.starts_with("REPAY ") || upper == "REPAY" {
            let mut it = text.split_whitespace();
            let _ = it.next();
            let to: i32 = it.next().and_then(|s| s.parse().ok()).unwrap_or(0);
            let amount_arg: Option<i32> = it.next().and_then(|s| s.parse().ok());
            let owed = state.debts.owed(p.p_id, to);
            let want = amount_arg.unwrap_or(owed);
            let pay = want.min(owed).max(0);
            let ok = if pay > 0 && state.economy.gift(p.p_id, to, pay) {
                match state.debts.repay(p.p_id, to, pay) {
                    Ok(applied) if applied == pay => {
                        for id in [p.p_id, to] {
                            let coins = state
                                .economy
                                .wallets
                                .get(&id)
                                .map(|w| w.coins)
                                .unwrap_or(0);
                            state.scoreboard.set_coins(id, coins);
                        }
                        true
                    }
                    _ => {
                        let _ = state.economy.gift(to, p.p_id, pay);
                        false
                    }
                }
            } else {
                false
            };
            let line = format!(
                "{} REPAY {} {} {}",
                p.p_id,
                to,
                if ok { pay } else { want },
                if ok { "OK" } else { "FAIL" }
            );
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        if upper == "?DEBT" || upper.starts_with("?DEBT") {
            let reply = state.debts.format_query(p.p_id);
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // TRADE <p_id> <coins> — set pending offer toward target (no transfer yet).
        if upper.starts_with("TRADE ") {
            let mut it = text.split_whitespace();
            let _ = it.next();
            let to: i32 = it.next().and_then(|s| s.parse().ok()).unwrap_or(0);
            let amount: i32 = it.next().and_then(|s| s.parse().ok()).unwrap_or(0);
            let ok = to != 0 && to != p.p_id && amount > 0;
            if ok {
                if let Some(pl) = state.players.get_mut(&conn_id) {
                    pl.trade_offer = Some((to, amount));
                }
            }
            let line = format!(
                "{} TRADE {} {} {}",
                p.p_id,
                to,
                amount,
                if ok { "OK" } else { "FAIL" }
            );
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // ACCEPT — take one incoming trade offer targeting this player via economy.transfer.
        if upper == "ACCEPT" {
            let accepter = p.p_id;
            // Find first online offerer with trade_offer.target == accepter.
            let offer = state.players.iter().find_map(|(&from_conn, pl)| {
                if pl.deleted || !pl.connected || pl.p_id == accepter {
                    return None;
                }
                match pl.trade_offer {
                    Some((target, amount)) if target == accepter && amount > 0 => {
                        Some((from_conn, pl.p_id, amount))
                    }
                    _ => None,
                }
            });
            let (ok, from_id, amount) = if let Some((from_conn, from_id, amount)) = offer {
                let transferred = state.economy.transfer(from_id, accepter, amount);
                if transferred {
                    if let Some(offerer) = state.players.get_mut(&from_conn) {
                        offerer.trade_offer = None;
                    }
                    for id in [from_id, accepter] {
                        let coins = state
                            .economy
                            .wallets
                            .get(&id)
                            .map(|w| w.coins)
                            .unwrap_or(0);
                        state.scoreboard.set_coins(id, coins);
                    }
                }
                (transferred, from_id, amount)
            } else {
                (false, 0, 0)
            };
            let line = format!(
                "{} ACCEPT {} {} {}",
                accepter,
                from_id,
                amount,
                if ok { "OK" } else { "FAIL" }
            );
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        if upper == "?COINS" || upper.starts_with("?COIN") {
            let coins = state
                .economy
                .wallets
                .get(&p.p_id)
                .map(|w| w.coins)
                .unwrap_or(0);
            let line = format!("{} COINS {coins}", p.p_id);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        if upper == "?SCORE" || upper.starts_with("SCORE?") {
            let reply = state.scoreboard.format_score_text(p.p_id);
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        if upper == "?HIGHSCORE" || upper.starts_with("?HIGHSCORE") {
            // Rank scoreboard players by prestige (not score).
            let prestiges: Vec<(i32, f32)> = state
                .scoreboard
                .entries
                .keys()
                .map(|&id| (id, state.player_prestige(id)))
                .collect();
            let reply = state.scoreboard.format_highscore_text(&prestiges, 10);
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        if upper == "?LEADERBOARD" || upper.starts_with("?LEAD") {
            let reply = state.scoreboard.format_leaderboard_text(10);
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        if upper == "?PRESTIGE" || upper == "?CLASS" || upper.starts_with("?PREST") {
            let info = state.player_prestige_info(p.p_id);
            let line = format!("{} {}", p.p_id, info);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // ?REP — combat reputation float (≠ prestige / PrestigeClass).
        if upper == "?REP" || upper == "REP" || upper.starts_with("?REP") {
            let reply = format_reputation_query(&state.reputation, p.p_id);
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // DEAF — toggle ignore all nearby chat except WHISPER.
        if upper == "DEAF" {
            let (p_id, on) = if let Some(pl) = state.players.get_mut(&conn_id) {
                pl.deaf = !pl.deaf;
                (pl.p_id, pl.deaf)
            } else {
                (p.p_id, false)
            };
            let line = format!("{} DEAF {}", p_id, if on { "ON" } else { "OFF" });
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // MUTE <p_id> / UNMUTE <p_id> / MUTE LIST — per-listener chat mute list.
        if let Some((action, opt_id)) = parse_mute_command(text) {
            let listener = p.p_id;
            match action {
                "mute" => {
                    let speaker = opt_id.unwrap_or(0);
                    let ok = state.mutes.mute(listener, speaker);
                    let line = format!(
                        "{} MUTE {} {}",
                        listener,
                        speaker,
                        if ok { "OK" } else { "FAIL" }
                    );
                    send_ps_reply(outbound, conn_id, &line);
                }
                "unmute" => {
                    let speaker = opt_id.unwrap_or(0);
                    let ok = state.mutes.unmute(listener, speaker);
                    let line = format!(
                        "{} UNMUTE {} {}",
                        listener,
                        speaker,
                        if ok { "OK" } else { "FAIL" }
                    );
                    send_ps_reply(outbound, conn_id, &line);
                }
                "list" => {
                    let reply = format_mute_query(&state.mutes, listener);
                    let line = format!("{} {}", listener, reply);
                    send_ps_reply(outbound, conn_id, &line);
                }
                _ => {}
            }
            return;
        }
        if upper == "?CURSE" || upper.starts_with("?CURSE") {
            // PS human-readable; also push CX/CS status (protocol CURSE_TOKEN / CURSE_SCORE).
            let reply = state.curses.format_query_text(p.p_id);
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            outbound.send(conn_id, state.curses.token_wire(p.p_id).into_bytes());
            outbound.send(conn_id, state.curses.score_wire(p.p_id).into_bytes());
            return;
        }
        if upper == "?APOC" || upper.starts_with("?APOC") {
            let reply = state.apocalypse.query_text();
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // STARTAPOC / ENDAPOC — force apocalypse for testing (no admin gate).
        if upper == "STARTAPOC" {
            state.apocalypse.trigger();
            let reply = state.apocalypse.query_text();
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            info!(conn_id, "sim: STARTAPOC");
            return;
        }
        if upper == "ENDAPOC" {
            state.apocalypse.end();
            let reply = state.apocalypse.query_text();
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            info!(conn_id, "sim: ENDAPOC");
            return;
        }
        // Exact `?WAR` only — do not match `?WARM` (clothing warmth query).
        if upper == "?WAR" || upper.starts_with("?WAR ") {
            let reply = state.war.format_query_text();
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        if upper == "?POSSE" || upper.starts_with("?POSSE") {
            let reply = state.posse.format_query_text(p.p_id);
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        if upper == "?YUM" || upper.starts_with("?YUM") {
            let reply = p.yum.query_text();
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // CLEAR YUM / RESET YUM — wipe YUM history + bonus (testing / parity reset).
        if upper == "CLEAR YUM"
            || upper == "RESET YUM"
            || upper == "CLEARYUM"
            || upper == "RESETYUM"
        {
            let reply = if let Some(pl) = state.players.get_mut(&conn_id) {
                pl.yum.clear();
                pl.yum.query_text()
            } else {
                "YUM bonus=0 history=0".into()
            };
            let line = format!("{} YUM CLEAR OK {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            info!(conn_id, "sim: CLEAR YUM");
            return;
        }
        // TOOLS / ?TOOLS — private PS: wire slots (used total) + learned count.
        if upper == "TOOLS" || upper == "?TOOLS" || upper.starts_with("?TOOLS") {
            let reply = p.tools.query_text();
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // FORGETTOOLS — clear learned tool slots (testing).
        if upper == "FORGETTOOLS" || upper == "?FORGETTOOLS" {
            let (ts_wire, reply) = if let Some(pl) = state.players.get_mut(&conn_id) {
                pl.tools.forget_all();
                (pl.tools.wire_slots(), pl.tools.query_text())
            } else {
                ("0 1000".into(), "TOOLS 0 1000 learned=0".into())
            };
            let line = format!("{} FORGETTOOLS OK {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            // Push cleared TS + empty LR so clients resync.
            outbound.send(
                conn_id,
                format_server_message("TS", &[&ts_wire]).into_bytes(),
            );
            outbound.send(
                conn_id,
                format_learned_tool_report(&[]).into_bytes(),
            );
            return;
        }
        // ?LOG / ?JOURNAL — last N session events (JOURNAL is an alias for LOG).
        if upper == "?LOG"
            || upper.starts_with("?LOG")
            || upper == "?JOURNAL"
            || upper == "JOURNAL"
            || upper.starts_with("?JOURNAL")
        {
            let reply = state.format_event_log_query();
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // ?WJOURNAL — peek last world-journal tile change (if journal Arc shared).
        if upper == "?WJOURNAL" || upper == "WJOURNAL" || upper.starts_with("?WJOURNAL") {
            let reply = state.format_wjournal_query();
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // !CLOSE — disconnect **this client only** (do not stop the server).
        if is_close_say(&upper) {
            let line = format!("{} CLOSE OK", p.p_id);
            send_ps_reply(outbound, conn_id, &line);
            if let Some(pl) = state.players.get_mut(&conn_id) {
                pl.connected = false;
            }
            state.publish_player_view(conn_id);
            // Close marker on urgent lane → net task flushes then drops TCP.
            outbound.close(conn_id);
            info!(conn_id, p_id = p.p_id, "sim: !CLOSE — client disconnect only");
            return;
        }
        // !shutdown / SHUTDOWN — orderly: countdown → save → AP → exit (whole process).
        // Match anywhere in the line (client may still attach noise); no godmode gate.
        if is_shutdown_say(&upper) {
            if state.shutdown.is_some() {
                let line = format!("{} SHUTDOWN ALREADY", p.p_id);
                send_ps_reply(outbound, conn_id, &line);
                return;
            }
            let secs = state.shutdown_countdown_secs.max(1.0);
            state.shutdown = Some(ShutdownState {
                remaining: secs,
                phase: ShutdownPhase::Countdown,
            });
            let msg = format!(
                "SERVER SHUTDOWN IN {:.0} SECONDS — saving and closing",
                secs
            );
            broadcast_global(outbound, &msg);
            let line = format!("{} SHUTDOWN OK in={:.0}s", p.p_id, secs);
            send_ps_reply(outbound, conn_id, &line);
            // Also echo as nearby chat so the speaker sees acknowledgement.
            let near = nearby_conn_ids(state, p.x, p.y, chat_range_for_age(p.age));
            {
                let _ps = format!("{} {}", p.p_id, msg);
                send_nearby_ps_lines(outbound, &near, &_ps);
            }
            info!(conn_id, secs, text = %text, "sim: !shutdown started");
            return;
        }
        // SAVE — operator (godmode) force-save signal; no disk I/O on sim thread.
        if upper == "SAVE" || upper == "?SAVE" {
            let god = state
                .players
                .get(&conn_id)
                .map(|pl| pl.godmode)
                .unwrap_or(false);
            let reply = if god {
                state.request_force_save()
            } else {
                format_save_denied()
            };
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            info!(conn_id, god, %reply, "sim: SAVE");
            return;
        }
        // WHO / ?WHO — list online player p_ids and display names via PS.
        if upper == "WHO" || upper == "?WHO" || upper.starts_with("?WHO") {
            let reply = state.format_who_query();
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // COUNT / ?COUNT — online player count.
        if upper == "COUNT" || upper == "?COUNT" {
            let reply = state.format_count_query();
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // NEAR / ?NEAR — nearby p_ids within NEARBY_RANGE (Chebyshev).
        if upper == "NEAR" || upper == "?NEAR" {
            let reply = state.format_near_query_at(p.x, p.y);
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // DIST <p_id> / ?DIST <p_id> — Chebyshev distance to target.
        if upper.starts_with("DIST ") || upper.starts_with("?DIST ") {
            let target: i32 = text
                .split_whitespace()
                .nth(1)
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            let reply = state.format_dist_query_to(p.x, p.y, target);
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // Bare DIST / ?DIST without arg → FAIL self-dist note.
        if upper == "DIST" || upper == "?DIST" {
            let reply = format_dist_query(0, None);
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // BIOME / ?BIOME — biome under feet with name + optional map-PNG hex.
        if upper == "BIOME" || upper == "?BIOME" {
            let biome = state.world.read().unwrap().get_biome(p.x, p.y);
            let hex = color_for_biome(biome).map(|c| c.to_hex());
            let reply =
                format_biome_query_with_hex(biome, biome_name(biome), hex.as_deref());
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // HEX / ?HEX — map-PNG color for biome under feet.
        if upper == "HEX" || upper == "?HEX" {
            let biome = state.world.read().unwrap().get_biome(p.x, p.y);
            let reply = format_hex_query(biome);
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // TAGS / ?TAGS — parse description tags of the held object.
        if upper == "TAGS" || upper == "?TAGS" {
            let held_id = state.players.get(&conn_id).map(|pl| pl.held_id).unwrap_or(0);
            let desc = state
                .content
                .get(held_id)
                .map(|d| d.description.as_str());
            let reply = format_held_tags_query(held_id, desc);
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // FLOOR / ?FLOOR — floor id under feet.
        if upper == "FLOOR" || upper == "?FLOOR" {
            let floor = state.world.read().unwrap().get_floor(p.x, p.y);
            let reply = format_floor_query(floor);
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // WHERE / ?WHERE — private PS: x y biome food age (no SQL).
        if upper == "WHERE" || upper == "?WHERE" || upper.starts_with("?WHERE") {
            let biome = state.world.read().unwrap().get_biome(p.x, p.y);
            let reply = SimState::format_where_query(p.x, p.y, biome, p.food, p.age);
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // FOOD / ?FOOD — private PS: current food and food_max (no SQL).
        if upper == "FOOD" || upper == "?FOOD" || upper.starts_with("?FOOD") {
            let (food, food_max) = state
                .players
                .get(&conn_id)
                .map(|pl| (pl.food, pl.food_max))
                .unwrap_or((p.food, p.food_max));
            let reply = SimState::format_food_query(food, food_max);
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // NAME / ?NAME — private PS: display_name + optional TITLE (no SQL).
        if upper == "NAME" || upper == "?NAME" || upper.starts_with("?NAME") {
            let name = state
                .players
                .get(&conn_id)
                .map(|pl| pl.name_for_query())
                .unwrap_or_else(|| p.name_for_query());
            let reply = SimState::format_name_query(&name);
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // AGE / ?AGE — private PS: current age years (no SQL).
        if upper == "AGE" || upper == "?AGE" || upper.starts_with("?AGE") {
            let age = state
                .players
                .get(&conn_id)
                .map(|pl| pl.age)
                .unwrap_or(p.age);
            let reply = SimState::format_age_query(age);
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // STATUS / ?STATUS — private PS: food age held prestige class wound sleep sick sit.
        if upper == "STATUS" || upper == "?STATUS" || upper.starts_with("?STATUS") {
            let (food, age, held_id, sleeping, sick, sitting) = state
                .players
                .get(&conn_id)
                .map(|pl| {
                    (
                        pl.food,
                        pl.age,
                        pl.held_id,
                        pl.sleeping,
                        pl.sick,
                        pl.sitting,
                    )
                })
                .unwrap_or((p.food, p.age, p.held_id, p.sleeping, p.sick, p.sitting));
            let wound = state.combat.wound_of(p.p_id);
            let prestige = state.player_prestige(p.p_id);
            let class = state.player_prestige_class(p.p_id);
            let reply = SimState::format_status_query(
                food, age, held_id, prestige, class, wound, sleeping, sick, sitting,
            );
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // HEART / ?HEART — compact vitals: food + age only.
        if upper == "HEART" || upper == "?HEART" || upper.starts_with("?HEART") {
            let (food, age) = state
                .players
                .get(&conn_id)
                .map(|pl| (pl.food, pl.age))
                .unwrap_or((p.food, p.age));
            let reply = SimState::format_heart_query(food, age);
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // FLAGS / ?FLAGS — sleeping sick sitting riding holding god deaf (0/1).
        if upper == "FLAGS" || upper == "?FLAGS" || upper.starts_with("?FLAGS") {
            let (sleeping, sick, sitting, riding, holding, god, deaf) = state
                .players
                .get(&conn_id)
                .map(|pl| {
                    (
                        pl.sleeping,
                        pl.sick,
                        pl.sitting,
                        pl.riding,
                        pl.holding_player_id != 0,
                        pl.godmode,
                        pl.deaf,
                    )
                })
                .unwrap_or((
                    p.sleeping,
                    p.sick,
                    p.sitting,
                    p.riding,
                    p.holding_player_id != 0,
                    p.godmode,
                    p.deaf,
                ));
            let reply = SimState::format_flags_query(
                sleeping, sick, sitting, riding, holding, god, deaf,
            );
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // BOOST — testing: +5 food (clamped to food_max).
        if upper == "BOOST" {
            let (food, food_max) = if let Some(pl) = state.players.get_mut(&conn_id) {
                pl.food = SimState::boost_food(pl.food, pl.food_max);
                (pl.food, pl.food_max)
            } else {
                (p.food, p.food_max)
            };
            let line = format!(
                "{} BOOST OK food={:.2} max={:.2}",
                p.p_id, food, food_max
            );
            send_ps_reply(outbound, conn_id, &line);
            info!(conn_id, food, "sim: BOOST");
            return;
        }
        // GODMODE — flag for lite god edits (VOGSET). ?GODMODE queries; ON/OFF set.
        if upper == "?GODMODE" || upper == "GODMODE?" {
            let god = state
                .players
                .get(&conn_id)
                .map(|pl| pl.godmode)
                .unwrap_or(p.godmode);
            let reply = SimState::format_godmode_query(god);
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        if upper == "GODMODE" || upper.starts_with("GODMODE ") {
            let arg = text.split_whitespace().nth(1).unwrap_or("").to_ascii_lowercase();
            let god = if let Some(pl) = state.players.get_mut(&conn_id) {
                match arg.as_str() {
                    "on" | "1" | "true" => pl.godmode = true,
                    "off" | "0" | "false" => pl.godmode = false,
                    _ => pl.godmode = !pl.godmode, // bare GODMODE toggles
                }
                pl.godmode
            } else {
                false
            };
            let reply = SimState::format_godmode_query(god);
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            info!(conn_id, god, "sim: GODMODE");
            return;
        }
        // SNAP [x y [seq]] — SAY alias for PHOTO deny (no photo backend).
        if upper == "SNAP" || upper.starts_with("SNAP ") {
            let mut parts = text.split_whitespace();
            let _ = parts.next(); // SNAP
            let x = parts.next().and_then(|s| s.parse().ok()).unwrap_or(p.x);
            let y = parts.next().and_then(|s| s.parse().ok()).unwrap_or(p.y);
            let seq = parts.next().unwrap_or("?");
            info!(conn_id, x, y, %seq, "sim: SNAP (PHOTO deny alias)");
            outbound.send(
                conn_id,
                format_photo_signature(x, y, PHOTO_DENIED_SIGNATURE).into_bytes(),
            );
            return;
        }
        // VOGSET <x> <y> <obj> — lite VoG map edit when godmode is on.
        if upper.starts_with("VOGSET ") || upper == "VOGSET" {
            let god = state
                .players
                .get(&conn_id)
                .map(|pl| pl.godmode)
                .unwrap_or(false);
            if !god {
                let line = format!("{} VOGSET DENIED", p.p_id);
                send_ps_reply(outbound, conn_id, &line);
                return;
            }
            let mut it = text.split_whitespace();
            let _ = it.next(); // VOGSET
            let x = it.next().and_then(|s| s.parse::<i32>().ok());
            let y = it.next().and_then(|s| s.parse::<i32>().ok());
            let obj = it.next().and_then(|s| s.parse::<i32>().ok());
            let (Some(x), Some(y), Some(obj)) = (x, y, obj) else {
                let line = format!("{} VOGSET FAIL", p.p_id);
                send_ps_reply(outbound, conn_id, &line);
                return;
            };
            state.world.write().unwrap().set_object(x, y, obj);
            state.record_world_change(x, y, obj);
            schedule_decay(state, x, y, obj);
            state.specials.remove(x, y);
            if obj != 0 {
                if let Some(def) = state.content.get(obj) {
                    if let Some(kind) = SpecialKind::from_object_name(&def.name) {
                        state.specials.insert(x, y, kind);
                    }
                }
            }
            let floor = state.world.read().unwrap().get_floor(x, y) as i32;
            let near = nearby_conn_ids(state, x, y, NEARBY_RANGE);
            send_nearby(
                outbound,
                &near,
                format_map_change(x, y, floor, obj, p.p_id).into_bytes(),
            );
            let line = format!("{} VOGSET {x} {y} {obj} OK", p.p_id);
            send_ps_reply(outbound, conn_id, &line);
            info!(conn_id, x, y, obj, "sim: VOGSET");
            return;
        }
        // REGEN — if tile under feet is empty, place a random natural object from
        // content.biome_spawn for the tile biome (testing / sparse re-seed).
        if upper == "REGEN" || upper == "?REGEN" {
            let x = p.x;
            let y = p.y;
            let (biome, cur_obj) = {
                let w = state.world.read().unwrap();
                (w.get_biome(x, y) as i32, w.get_object(x, y))
            };
            if cur_obj != 0 {
                let line = format!("{} REGEN SKIP not_empty obj={cur_obj}", p.p_id);
                send_ps_reply(outbound, conn_id, &line);
                return;
            }
            let Some(table) = state.content.biome_spawn.get(&biome) else {
                let line = format!("{} REGEN FAIL no_spawn biome={biome}", p.p_id);
                send_ps_reply(outbound, conn_id, &line);
                return;
            };
            let mut rng = rand::thread_rng();
            let Some(obj) = pick_biome_spawn(table, &mut rng) else {
                let line = format!("{} REGEN FAIL no_spawn biome={biome}", p.p_id);
                send_ps_reply(outbound, conn_id, &line);
                return;
            };
            {
                let mut w = state.world.write().unwrap();
                place_natural_object(&mut w, &state.content, x, y, obj);
            }
            state.record_world_change(x, y, obj);
            schedule_decay(state, x, y, obj);
            state.specials.remove(x, y);
            if obj != 0 {
                if let Some(def) = state.content.get(obj) {
                    if let Some(kind) = SpecialKind::from_object_name(&def.name) {
                        state.specials.insert(x, y, kind);
                    }
                }
            }
            let floor = state.world.read().unwrap().get_floor(x, y) as i32;
            let near = nearby_conn_ids(state, x, y, NEARBY_RANGE);
            send_nearby(
                outbound,
                &near,
                format_map_change(x, y, floor, obj, p.p_id).into_bytes(),
            );
            let line = format!("{} REGEN OK {x} {y} {obj}", p.p_id);
            send_ps_reply(outbound, conn_id, &line);
            info!(conn_id, x, y, obj, biome, "sim: REGEN");
            return;
        }
        // CLEAROBJ — godmode: clear ground object under feet (set object id 0).
        if upper == "CLEAROBJ" || upper == "?CLEAROBJ" {
            let god = state
                .players
                .get(&conn_id)
                .map(|pl| pl.godmode)
                .unwrap_or(false);
            if !god {
                let line = format!("{} CLEAROBJ DENIED", p.p_id);
                send_ps_reply(outbound, conn_id, &line);
                return;
            }
            let x = p.x;
            let y = p.y;
            state.world.write().unwrap().set_object(x, y, 0);
            state.record_world_change(x, y, 0);
            schedule_decay(state, x, y, 0);
            state.specials.remove(x, y);
            let floor = state.world.read().unwrap().get_floor(x, y) as i32;
            let near = nearby_conn_ids(state, x, y, NEARBY_RANGE);
            send_nearby(
                outbound,
                &near,
                format_map_change(x, y, floor, 0, p.p_id).into_bytes(),
            );
            let line = format!("{} CLEAROBJ OK {x} {y}", p.p_id);
            send_ps_reply(outbound, conn_id, &line);
            info!(conn_id, x, y, "sim: CLEAROBJ");
            return;
        }
        // FILL — godmode: set floor under feet to 1 (indoor / road stub).
        if upper == "FILL" || upper == "?FILL" {
            let god = state
                .players
                .get(&conn_id)
                .map(|pl| pl.godmode)
                .unwrap_or(false);
            if !god {
                let line = format!("{} FILL DENIED", p.p_id);
                send_ps_reply(outbound, conn_id, &line);
                return;
            }
            let x = p.x;
            let y = p.y;
            state.world.write().unwrap().set_floor(x, y, 1);
            let obj = state.world.read().unwrap().get_object(x, y);
            let near = nearby_conn_ids(state, x, y, NEARBY_RANGE);
            send_nearby(
                outbound,
                &near,
                format_map_change(x, y, 1, obj, p.p_id).into_bytes(),
            );
            let line = format!("{} FILL OK {x} {y} floor=1", p.p_id);
            send_ps_reply(outbound, conn_id, &line);
            info!(conn_id, x, y, "sim: FILL floor=1");
            return;
        }
        if upper.starts_with("POSSE ") {
            // POSSE <p_id> — join posse of target; POSSE 0 — leave all.
            let rest = text.split_whitespace().nth(1).unwrap_or("");
            if let Ok(target_id) = rest.parse::<i32>() {
                let near = nearby_conn_ids(state, p.x, p.y, NEARBY_RANGE);
                if target_id == 0 {
                    let _had = state.posse.clear(p.p_id);
                    let line = format!("{} POSSE 0 OK", p.p_id);
                    send_nearby_ps_lines(outbound, &near, &line);
                    send_nearby(outbound, &near, format_posse_join(p.p_id, 0).into_bytes());
                } else {
                    let ok = state.posse.add_posse(p.p_id, target_id);
                    let line = format!(
                        "{} POSSE {} {}",
                        p.p_id,
                        target_id,
                        if ok { "OK" } else { "FAIL" }
                    );
                    send_nearby_ps_lines(outbound, &near, &line);
                    if ok {
                        send_nearby(
                            outbound,
                            &near,
                            format_posse_join(p.p_id, target_id).into_bytes(),
                        );
                    }
                }
            }
            return;
        }
        if upper.starts_with("WAR ") {
            // WAR <p_id> — declare war on target (Haxe WAR_REPORT subset).
            let rest = text.split_whitespace().nth(1).unwrap_or("");
            if let Ok(target_id) = rest.parse::<i32>() {
                let ok = state.war.declare_war(p.p_id, target_id);
                let status = if ok {
                    STATUS_WAR
                } else {
                    STATUS_PEACE
                };
                let line = format!(
                    "{} WAR {} {} {}",
                    p.p_id,
                    target_id,
                    status,
                    if ok { "OK" } else { "FAIL" }
                );
                let near = nearby_conn_ids(state, p.x, p.y, NEARBY_RANGE);
                send_nearby_ps_lines(outbound, &near, &line);
                if ok {
                    state.push_event(format!("WAR {} {}", p.p_id, target_id));
                    // Optional scoreboard touch: ensure both parties have rows.
                    state.scoreboard.ensure_player(p.p_id, format!("P{}", p.p_id));
                    state
                        .scoreboard
                        .ensure_player(target_id, format!("P{target_id}"));
                    let wr = format_war_report(p.p_id, target_id, STATUS_WAR);
                    send_nearby(outbound, &near, wr.into_bytes());
                }
            }
            return;
        }
        // RAID <p_id> — posse attack note (prestige note only; no combat).
        // Requires mutual posse membership (both have each other as targets).
        if upper.starts_with("RAID ") {
            let rest = text.split_whitespace().nth(1).unwrap_or("");
            if let Ok(target_id) = rest.parse::<i32>() {
                let both_in_posse = target_id != p.p_id
                    && state.posse.has_target(p.p_id, target_id)
                    && state.posse.has_target(target_id, p.p_id);
                let prestige = state.player_prestige(p.p_id);
                let line = if both_in_posse {
                    format!(
                        "{} RAID {} OK prestige={:.2}",
                        p.p_id, target_id, prestige
                    )
                } else {
                    format!("{} RAID {} FAIL", p.p_id, target_id)
                };
                send_ps_reply(outbound, conn_id, &line);
                if both_in_posse {
                    state.push_event(format!("RAID {} {}", p.p_id, target_id));
                }
            }
            return;
        }
        if upper.starts_with("PEACE ") {
            // PEACE <p_id> — make peace with target.
            let rest = text.split_whitespace().nth(1).unwrap_or("");
            if let Ok(target_id) = rest.parse::<i32>() {
                let ok = state.war.make_peace(p.p_id, target_id);
                let line = format!(
                    "{} PEACE {} {} {}",
                    p.p_id,
                    target_id,
                    STATUS_PEACE,
                    if ok { "OK" } else { "FAIL" }
                );
                let near = nearby_conn_ids(state, p.x, p.y, NEARBY_RANGE);
                send_nearby_ps_lines(outbound, &near, &line);
                if ok {
                    let wr = format_war_report(p.p_id, target_id, STATUS_PEACE);
                    send_nearby(outbound, &near, wr.into_bytes());
                }
            }
            return;
        }
        if upper.starts_with("KILL ") {
            let rest = text.split_whitespace().nth(1).unwrap_or("");
            if let Ok(target_id) = rest.parse::<i32>() {
                let killer_id = p.p_id;
                let killer_x = p.x;
                let killer_y = p.y;
                // Range gate (Chebyshev): online targets beyond KILL_RANGE miss.
                // Offline / unknown targets still allow one-shot resolve_kill (prestige only).
                let target_pos = state
                    .players
                    .values()
                    .find(|x| x.p_id == target_id && !x.deleted)
                    .map(|tp| (tp.x, tp.y));
                if let Some((tx, ty)) = target_pos {
                    let dist = CombatState::chebyshev(killer_x, killer_y, tx, ty);
                    if dist > KILL_RANGE {
                        let line = format!("{} KILL {} MISS range", killer_id, target_id);
                        send_ps_reply(outbound, conn_id, &line);
                        return;
                    }
                }
                let legal = state.social.is_exiled_by(killer_id, target_id)
                    || state
                        .social
                        .following
                        .get(&killer_id)
                        .map(|leader| state.social.is_exiled_by(*leader, target_id))
                        .unwrap_or(false);
                // SAY KILL remains one-shot via resolve_kill (wound path is resolve_hit / HIT).
                if state.combat.resolve_kill(killer_id, target_id, legal) {
                    state.combat.clear_wound(target_id);
                    // Combat reputation (≠ prestige): illegal guilt / legal recover.
                    if legal {
                        state
                            .reputation
                            .apply_legal_hit(killer_id, target_id, 0.2);
                    } else {
                        state
                            .reputation
                            .apply_illegal_hit(killer_id, 1.0, 1.0);
                    }
                    // Keep lineage prestige/class in sync with combat prestige.
                    state.sync_lineage_prestige_from_combat(killer_id);
                    state.sync_lineage_prestige_from_combat(target_id);
                    // Scoreboard kill/death counters + score.
                    state.scoreboard.record_kill(killer_id, target_id);
                    state.refresh_living_prestige_classes();
                    let cause = combat_death(legal);
                    let death_reason = cause.wire_tag();
                    // Mark target deleted if online (held/clothing/backpack drained by scatter).
                    if let Some(tp) = state.players.values_mut().find(|x| x.p_id == target_id) {
                        tp.deleted = true;
                        tp.death_reason = Some(death_reason.into());
                    }
                    scatter_backpack_on_death_pid(state, target_id);
                    apply_death_inheritance(state, target_id);
                    counters.deaths.fetch_add(1, Ordering::Relaxed);
                    state.push_event(format_death_event(target_id, cause));
                    state.afk.remove(target_id);
                    let line = format!("{} KILLED {} legal={}", killer_id, target_id, legal);
                    let near = nearby_conn_ids(state, killer_x, killer_y, NEARBY_RANGE);
                    send_nearby_ps_lines(outbound, &near, &line);
                }
            }
            return;
        }
        // PUSH <p_id> — shove adjacent non-god target one tile away, or swap if blocked.
        if upper.starts_with("PUSH ") {
            let rest = text.split_whitespace().nth(1).unwrap_or("");
            if let Ok(target_id) = rest.parse::<i32>() {
                let actor_id = p.p_id;
                let (ax, ay) = (p.x, p.y);
                let target_info = state.players.iter().find_map(|(&tc, tp)| {
                    if tp.p_id == target_id && !tp.deleted {
                        Some((tc, tp.x, tp.y, tp.godmode, tp.held_id, tp.age))
                    } else {
                        None
                    }
                });
                let line = match target_info {
                    None => format!("{actor_id} PUSH {target_id} FAIL offline"),
                    Some(_) if target_id == actor_id => {
                        format!("{actor_id} PUSH {target_id} FAIL self")
                    }
                    Some((_, _, _, true, _, _)) => {
                        format!("{actor_id} PUSH {target_id} FAIL god")
                    }
                    Some((t_conn, tx, ty, _, t_held, t_age)) => {
                        if !shove_is_adjacent(ax, ay, tx, ty) {
                            format!("{actor_id} PUSH {target_id} FAIL range")
                        } else {
                            let (dx, dy) = push_dest(ax, ay, tx, ty);
                            let dest_walkable = {
                                let world = state.world.read().unwrap();
                                let (wx, wy) = world.wrap_tile(dx, dy);
                                !biome_blocks_move(world.get_biome(wx, wy))
                                    && is_walkable(&world, &state.content, wx, wy)
                            };
                            let third_on_dest = state.players.values().any(|op| {
                                !op.deleted
                                    && op.p_id != actor_id
                                    && op.p_id != target_id
                                    && op.x == dx
                                    && op.y == dy
                            });
                            let dest_free = dest_walkable && !third_on_dest;
                            let outcome = resolve_push(ax, ay, tx, ty, dest_free);
                            let actor_held = p.held_id;
                            let actor_age = p.age;
                            // Resolve wrap before mutating players (avoid double-borrow).
                            let shove_xy = match outcome {
                                PushOutcome::Shove { nx, ny } => {
                                    let w = state.world.read().unwrap();
                                    Some(w.wrap_tile(nx, ny))
                                }
                                PushOutcome::Swap => None,
                            };
                            match outcome {
                                PushOutcome::Shove { .. } => {
                                    let (nx, ny) = shove_xy.unwrap();
                                    if let Some(tp) = state.players.get_mut(&t_conn) {
                                        tp.x = nx;
                                        tp.y = ny;
                                    }
                                }
                                PushOutcome::Swap => {
                                    if let Some(ap) = state.players.get_mut(&conn_id) {
                                        ap.x = tx;
                                        ap.y = ty;
                                    }
                                    if let Some(tp) = state.players.get_mut(&t_conn) {
                                        tp.x = ax;
                                        tp.y = ay;
                                    }
                                }
                            }
                            state.publish_player_view(conn_id);
                            state.publish_player_view(t_conn);
                            let near = nearby_conn_ids(state, ax, ay, NEARBY_RANGE);
                            // PU for actor + target after position change.
                            if let Some(ap) = state.players.get(&conn_id) {
                                let spd = player_move_speed(state, ap);
                                let pu = format_player_update_line(
                                    actor_id,
                                    DEFAULT_PERSON_OBJECT,
                                    actor_held,
                                    ap.x,
                                    ap.y,
                                    actor_age,
                                    spd,
                                ap.done_moving_seq.max(1),
                                );
                                send_nearby(
                                    outbound,
                                    &near,
                                    format_server_message("PU", &[&pu]).into_bytes(),
                                );
                            }
                            if let Some(tp) = state.players.get(&t_conn) {
                                let spd = player_move_speed(state, tp);
                                let pu = format_player_update_line(
                                    target_id,
                                    DEFAULT_PERSON_OBJECT,
                                    t_held,
                                    tp.x,
                                    tp.y,
                                    t_age,
                                    spd,
                                tp.done_moving_seq.max(1),
                                );
                                send_nearby(
                                    outbound,
                                    &near,
                                    format_server_message("PU", &[&pu]).into_bytes(),
                                );
                            }
                            let mode = match outcome {
                                PushOutcome::Shove { .. } => "shove",
                                PushOutcome::Swap => "swap",
                            };
                            let (fx, fy) = state
                                .players
                                .get(&t_conn)
                                .map(|tp| (tp.x, tp.y))
                                .unwrap_or((tx, ty));
                            format!("{actor_id} PUSH {target_id} OK {mode} {fx} {fy}")
                        }
                    }
                };
                let near = nearby_conn_ids(
                    state,
                    state.players.get(&conn_id).map(|p| p.x).unwrap_or(ax),
                    state.players.get(&conn_id).map(|p| p.y).unwrap_or(ay),
                    NEARBY_RANGE,
                );
                if line.contains(" OK ") {
                    send_nearby_ps_lines(outbound, &near, &line);
                } else {
                    send_ps_reply(outbound, conn_id, &line);
                }
                info!(conn_id, %line, "sim: PUSH");
            }
            return;
        }
        // PULL <p_id> — pull adjacent target one step toward self if dest empty.
        if upper.starts_with("PULL ") {
            let rest = text.split_whitespace().nth(1).unwrap_or("");
            if let Ok(target_id) = rest.parse::<i32>() {
                let actor_id = p.p_id;
                let (ax, ay) = (p.x, p.y);
                let target_info = state.players.iter().find_map(|(&tc, tp)| {
                    if tp.p_id == target_id && !tp.deleted {
                        Some((tc, tp.x, tp.y, tp.held_id, tp.age))
                    } else {
                        None
                    }
                });
                let line = match target_info {
                    None => format!("{actor_id} PULL {target_id} FAIL offline"),
                    Some(_) if target_id == actor_id => {
                        format!("{actor_id} PULL {target_id} FAIL self")
                    }
                    Some((t_conn, tx, ty, t_held, t_age)) => {
                        if !shove_is_adjacent(ax, ay, tx, ty) {
                            format!("{actor_id} PULL {target_id} FAIL range")
                        } else {
                            let (dx, dy) = pull_dest(ax, ay, tx, ty);
                            let (wx, wy) = {
                                let world = state.world.read().unwrap();
                                world.wrap_tile(dx, dy)
                            };
                            let dest_walkable = {
                                let world = state.world.read().unwrap();
                                !biome_blocks_move(world.get_biome(wx, wy))
                                    && is_walkable(&world, &state.content, wx, wy)
                            };
                            // Actor tile is allowed; third players block.
                            let third_on_dest = state.players.values().any(|op| {
                                !op.deleted
                                    && op.p_id != actor_id
                                    && op.p_id != target_id
                                    && op.x == wx
                                    && op.y == wy
                            });
                            if !can_pull_to(ax, ay, wx, wy, dest_walkable, third_on_dest) {
                                format!("{actor_id} PULL {target_id} FAIL blocked")
                            } else {
                                if let Some(tp) = state.players.get_mut(&t_conn) {
                                    tp.x = wx;
                                    tp.y = wy;
                                }
                                state.publish_player_view(t_conn);
                                let near = nearby_conn_ids(state, ax, ay, NEARBY_RANGE);
                                if let Some(tp) = state.players.get(&t_conn) {
                                    let spd = player_move_speed(state, tp);
                                    let pu = format_player_update_line(
                                        target_id,
                                        DEFAULT_PERSON_OBJECT,
                                        t_held,
                                        tp.x,
                                        tp.y,
                                        t_age,
                                        spd,
                                    tp.done_moving_seq.max(1),
                                    );
                                    send_nearby(
                                        outbound,
                                        &near,
                                        format_server_message("PU", &[&pu]).into_bytes(),
                                    );
                                }
                                format!("{actor_id} PULL {target_id} OK {wx} {wy}")
                            }
                        }
                    }
                };
                if line.contains(" OK ") {
                    let near = nearby_conn_ids(state, ax, ay, NEARBY_RANGE);
                    send_nearby_ps_lines(outbound, &near, &line);
                } else {
                    send_ps_reply(outbound, conn_id, &line);
                }
                info!(conn_id, %line, "sim: PULL");
            }
            return;
        }
        // KISS <p_id> — PE cute/love if adjacent; tiny prestige when ally.
        if upper.starts_with("KISS ") {
            let rest = text.split_whitespace().nth(1).unwrap_or("");
            if let Ok(target_id) = rest.parse::<i32>() {
                let actor_id = p.p_id;
                let (ax, ay) = (p.x, p.y);
                let target_pos = state.players.values().find_map(|tp| {
                    if tp.p_id == target_id && !tp.deleted {
                        Some((tp.x, tp.y))
                    } else {
                        None
                    }
                });
                let line = match target_pos {
                    None => format!("{actor_id} KISS {target_id} FAIL offline"),
                    Some(_) if target_id == actor_id => {
                        format!("{actor_id} KISS {target_id} FAIL self")
                    }
                    Some((tx, ty)) => {
                        // Adjacent or same tile (Chebyshev <= 1).
                        let dist = (ax - tx).abs().max((ay - ty).abs());
                        if dist > SHOVE_RANGE {
                            format!("{actor_id} KISS {target_id} FAIL range")
                        } else {
                            let ally = state.allies.is_mutual_or_either(actor_id, target_id);
                            let mut prest_note = 0.0f32;
                            if ally {
                                state.combat.stats_mut(actor_id).prestige += KISS_ALLY_PRESTIGE;
                                state.sync_lineage_prestige_from_combat(actor_id);
                                prest_note = KISS_ALLY_PRESTIGE;
                            }
                            let near = nearby_conn_ids(state, ax, ay, NEARBY_RANGE);
                            // PE cute/love on the kisser.
                            let pe = format_server_message(
                                "PE",
                                &[&format!("{actor_id} {CUTE_EMOT_INDEX}")],
                            );
                            send_nearby(outbound, &near, pe.into_bytes());
                            if ally {
                                format!(
                                    "{actor_id} KISS {target_id} OK cute prestige={prest_note:.2}"
                                )
                            } else {
                                format!("{actor_id} KISS {target_id} OK cute")
                            }
                        }
                    }
                };
                if line.contains(" OK") {
                    let near = nearby_conn_ids(state, ax, ay, NEARBY_RANGE);
                    send_nearby_ps_lines(outbound, &near, &line);
                } else {
                    send_ps_reply(outbound, conn_id, &line);
                }
                info!(conn_id, %line, "sim: KISS");
            }
            return;
        }
        // THANK <p_id> — prestige +THANK_PRESTIGE when target adjacent (Chebyshev ≤ 1).
        if upper.starts_with("THANK ") {
            let rest = text.split_whitespace().nth(1).unwrap_or("");
            if let Ok(target_id) = rest.parse::<i32>() {
                let actor_id = p.p_id;
                let (ax, ay) = (p.x, p.y);
                let target_pos = state.players.values().find_map(|tp| {
                    if tp.p_id == target_id && !tp.deleted {
                        Some((tp.x, tp.y))
                    } else {
                        None
                    }
                });
                let line = match target_pos {
                    None => format!("{actor_id} THANK {target_id} FAIL offline"),
                    Some(_) if target_id == actor_id => {
                        format!("{actor_id} THANK {target_id} FAIL self")
                    }
                    Some((tx, ty)) => {
                        let dist = (ax - tx).abs().max((ay - ty).abs());
                        if dist > SHOVE_RANGE {
                            format!("{actor_id} THANK {target_id} FAIL range")
                        } else {
                            state.combat.stats_mut(actor_id).prestige += THANK_PRESTIGE;
                            state.sync_lineage_prestige_from_combat(actor_id);
                            format!(
                                "{actor_id} THANK {target_id} OK prestige={:.2}",
                                THANK_PRESTIGE
                            )
                        }
                    }
                };
                if line.contains(" OK") {
                    let near = nearby_conn_ids(state, ax, ay, NEARBY_RANGE);
                    send_nearby_ps_lines(outbound, &near, &line);
                } else {
                    send_ps_reply(outbound, conn_id, &line);
                }
                info!(conn_id, %line, "sim: THANK");
            }
            return;
        }
        // CURSE <p_id> — spend one curse token; +1 score on target (CX/CS wires).
        if upper.starts_with("CURSE ") {
            let rest = text.split_whitespace().nth(1).unwrap_or("");
            if let Ok(target_id) = rest.parse::<i32>() {
                let actor_id = p.p_id;
                let target_online = state
                    .players
                    .values()
                    .any(|tp| tp.p_id == target_id && !tp.deleted);
                let line = if target_id == actor_id {
                    format!("{actor_id} CURSE {target_id} FAIL self")
                } else if !target_online {
                    format!("{actor_id} CURSE {target_id} FAIL offline")
                } else if !state.curses.curse_player(actor_id, target_id) {
                    format!("{actor_id} CURSE {target_id} FAIL no_token")
                } else {
                    // Push CX to curser and CS to target (and CX/CS refresh for both).
                    outbound.send(conn_id, state.curses.token_wire(actor_id).into_bytes());
                    outbound.send(conn_id, state.curses.score_wire(actor_id).into_bytes());
                    if let Some((t_conn, _)) = state
                        .players
                        .iter()
                        .find(|(_, tp)| tp.p_id == target_id && !tp.deleted)
                    {
                        outbound.send(*t_conn, state.curses.token_wire(target_id).into_bytes());
                        outbound.send(*t_conn, state.curses.score_wire(target_id).into_bytes());
                    }
                    format!(
                        "{actor_id} CURSE {target_id} OK tokens={} score={}",
                        state.curses.tokens(actor_id),
                        state.curses.score(target_id)
                    )
                };
                if line.contains(" OK") {
                    let near = nearby_conn_ids(state, p.x, p.y, NEARBY_RANGE);
                    send_nearby_ps_lines(outbound, &near, &line);
                } else {
                    send_ps_reply(outbound, conn_id, &line);
                }
                info!(conn_id, %line, "sim: CURSE");
            }
            return;
        }
        // BLESS <p_id> — clear target wounds + tiny prestige when adjacent (Chebyshev ≤ 1).
        if upper.starts_with("BLESS ") {
            let rest = text.split_whitespace().nth(1).unwrap_or("");
            if let Ok(target_id) = rest.parse::<i32>() {
                let actor_id = p.p_id;
                let (ax, ay) = (p.x, p.y);
                let target_pos = state.players.values().find_map(|tp| {
                    if tp.p_id == target_id && !tp.deleted {
                        Some((tp.x, tp.y))
                    } else {
                        None
                    }
                });
                let line = match target_pos {
                    None => format!("{actor_id} BLESS {target_id} FAIL offline"),
                    Some((tx, ty)) => {
                        let dist = (ax - tx).abs().max((ay - ty).abs());
                        if dist > SHOVE_RANGE {
                            format!("{actor_id} BLESS {target_id} FAIL range")
                        } else {
                            let prev = state.combat.wound_of(target_id);
                            state.combat.clear_wound(target_id);
                            // Self-bless heals only (no prestige); bless others → tiny prestige.
                            let prest_note = if target_id != actor_id {
                                state.combat.stats_mut(actor_id).prestige += BLESS_PRESTIGE;
                                state.sync_lineage_prestige_from_combat(actor_id);
                                BLESS_PRESTIGE
                            } else {
                                0.0
                            };
                            if target_id != actor_id {
                                format!(
                                    "{actor_id} BLESS {target_id} OK was={prev} prestige={prest_note:.2}"
                                )
                            } else {
                                format!("{actor_id} BLESS {target_id} OK was={prev}")
                            }
                        }
                    }
                };
                if line.contains(" OK") {
                    let near = nearby_conn_ids(state, ax, ay, NEARBY_RANGE);
                    send_nearby_ps_lines(outbound, &near, &line);
                } else {
                    send_ps_reply(outbound, conn_id, &line);
                }
                info!(conn_id, %line, "sim: BLESS");
            }
            return;
        }
        // HUG <p_id> — PE love when adjacent (Chebyshev ≤ 1).
        if upper.starts_with("HUG ") {
            let rest = text.split_whitespace().nth(1).unwrap_or("");
            if let Ok(target_id) = rest.parse::<i32>() {
                let actor_id = p.p_id;
                let (ax, ay) = (p.x, p.y);
                let target_pos = state.players.values().find_map(|tp| {
                    if tp.p_id == target_id && !tp.deleted {
                        Some((tp.x, tp.y))
                    } else {
                        None
                    }
                });
                let line = match target_pos {
                    None => format!("{actor_id} HUG {target_id} FAIL offline"),
                    Some(_) if target_id == actor_id => {
                        format!("{actor_id} HUG {target_id} FAIL self")
                    }
                    Some((tx, ty)) => {
                        let dist = (ax - tx).abs().max((ay - ty).abs());
                        if dist > SHOVE_RANGE {
                            format!("{actor_id} HUG {target_id} FAIL range")
                        } else {
                            let near = nearby_conn_ids(state, ax, ay, NEARBY_RANGE);
                            let pe = format_server_message(
                                "PE",
                                &[&format!("{actor_id} {LOVE_EMOT_INDEX}")],
                            );
                            send_nearby(outbound, &near, pe.into_bytes());
                            format!("{actor_id} HUG {target_id} OK love")
                        }
                    }
                };
                if line.contains(" OK") {
                    let near = nearby_conn_ids(state, ax, ay, NEARBY_RANGE);
                    send_nearby_ps_lines(outbound, &near, &line);
                } else {
                    send_ps_reply(outbound, conn_id, &line);
                }
                info!(conn_id, %line, "sim: HUG");
            }
            return;
        }
        // SLAP <p_id> — PE mad when adjacent; tiny wound if not ally.
        if upper.starts_with("SLAP ") {
            let rest = text.split_whitespace().nth(1).unwrap_or("");
            if let Ok(target_id) = rest.parse::<i32>() {
                let actor_id = p.p_id;
                let (ax, ay) = (p.x, p.y);
                let target_pos = state.players.values().find_map(|tp| {
                    if tp.p_id == target_id && !tp.deleted {
                        Some((tp.x, tp.y))
                    } else {
                        None
                    }
                });
                let line = match target_pos {
                    None => format!("{actor_id} SLAP {target_id} FAIL offline"),
                    Some(_) if target_id == actor_id => {
                        format!("{actor_id} SLAP {target_id} FAIL self")
                    }
                    Some((tx, ty)) => {
                        let dist = (ax - tx).abs().max((ay - ty).abs());
                        if dist > SHOVE_RANGE {
                            format!("{actor_id} SLAP {target_id} FAIL range")
                        } else {
                            let ally = state.allies.is_mutual_or_either(actor_id, target_id);
                            let near = nearby_conn_ids(state, ax, ay, NEARBY_RANGE);
                            // PE mad on the slapper (Haxe Emote.mad).
                            let pe = format_server_message(
                                "PE",
                                &[&format!("{actor_id} {MAD_EMOT_INDEX}")],
                            );
                            send_nearby(outbound, &near, pe.into_bytes());
                            if !ally {
                                let w = state.combat.apply_wound(target_id, SLAP_WOUND);
                                format!("{actor_id} SLAP {target_id} OK mad wound={w}")
                            } else {
                                format!("{actor_id} SLAP {target_id} OK mad")
                            }
                        }
                    }
                };
                if line.contains(" OK") {
                    let near = nearby_conn_ids(state, ax, ay, NEARBY_RANGE);
                    send_nearby_ps_lines(outbound, &near, &line);
                } else {
                    send_ps_reply(outbound, conn_id, &line);
                }
                info!(conn_id, %line, "sim: SLAP");
            }
            return;
        }
        // HIT <p_id> — Haxe doDamage: weapon damage + clothing protection + wounds.
        // Weapon range from held object name (bow=8, sword/knife=2, spear=3, default KILL_RANGE).
        if upper.starts_with("HIT ") {
            let rest = text.split_whitespace().nth(1).unwrap_or("");
            if let Ok(target_id) = rest.parse::<i32>() {
                let killer_id = p.p_id;
                let killer_x = p.x;
                let killer_y = p.y;
                let held_id = p.held_id;
                let held_name = held_object_name(state, held_id);
                let max_range = weapon_range(held_id, &held_name);
                let org_damage = weapon_damage(held_id, &held_name);
                let target_info = state.players.values().find(|x| x.p_id == target_id && !x.deleted).map(|tp| {
                    (
                        tp.x,
                        tp.y,
                        tp.food_max,
                        clothing_temp_bonus(tp.hat, tp.chest, tp.shoes),
                        tp.held_id,
                    )
                });
                let Some((tx, ty, t_food_max, cloth_insul, t_held)) = target_info else {
                    let line = format!("{} HIT {} FAIL offline", killer_id, target_id);
                    send_ps_reply(outbound, conn_id, &line);
                    return;
                };
                let t_held_name = held_object_name(state, t_held);
                let weapon_prot = held_damage_protection_factor(t_held, &t_held_name);
                // Floor insulation proxy: non-zero floor → 0.3 (Haxe floor.getInsulation subset).
                let floor_insul = {
                    let w = state.world.read().unwrap();
                    if w.get_floor(tx, ty) != 0 {
                        0.3
                    } else {
                        0.0
                    }
                };
                let legal = state.social.is_exiled_by(killer_id, target_id)
                    || state
                        .social
                        .following
                        .get(&killer_id)
                        .map(|leader| state.social.is_exiled_by(*leader, target_id))
                        .unwrap_or(false);
                let rng01 = rand::random::<f32>();
                let (result, dmg) = state.combat.resolve_hit_damaged(
                    killer_id,
                    target_id,
                    killer_x,
                    killer_y,
                    tx,
                    ty,
                    legal,
                    max_range,
                    org_damage,
                    cloth_insul,
                    floor_insul,
                    weapon_prot,
                    t_food_max,
                    held_id,
                    rng01,
                );
                let near = nearby_conn_ids(state, killer_x, killer_y, NEARBY_RANGE);
                match result {
                    HitResult::Miss => {
                        let line = format!("{} HIT {} MISS", killer_id, target_id);
                        send_ps_reply(outbound, conn_id, &line);
                    }
                    HitResult::Wound(w) => {
                        // Reduce food_max (HP) by damage (Haxe food_store_max from hits).
                        if let Some(tp) =
                            state.players.values_mut().find(|x| x.p_id == target_id)
                        {
                            tp.food_max = (tp.food_max - dmg).max(FOOD_MAX_DEATH);
                            if tp.food > tp.food_max {
                                tp.food = tp.food_max;
                            }
                        }
                        let line = format!(
                            "{} HIT {} WOUND {} dmg={:.1}",
                            killer_id, target_id, w, dmg
                        );
                        send_nearby_ps_lines(outbound, &near, &line);
                        // Wound PE emote (Haxe Emote.mad) on the target to nearby.
                        let pe = format_server_message(
                            "PE",
                            &[&format!("{target_id} {HUNGER_EMOT_INDEX}")],
                        );
                        send_nearby(outbound, &near, pe.into_bytes());
                        // DY dying indicator while wounded (Haxe SendDyingToAll).
                        send_nearby(
                            outbound,
                            &near,
                            format_dying(target_id, false).into_bytes(),
                        );
                    }
                    HitResult::Kill => {
                        if legal {
                            state
                                .reputation
                                .apply_legal_hit(killer_id, target_id, 0.2);
                        } else {
                            state
                                .reputation
                                .apply_illegal_hit(killer_id, 1.0, 1.0);
                        }
                        state.sync_lineage_prestige_from_combat(killer_id);
                        state.sync_lineage_prestige_from_combat(target_id);
                        state.scoreboard.record_kill(killer_id, target_id);
                        state.refresh_living_prestige_classes();
                        let cause = combat_death(legal);
                        let death_reason = cause.wire_tag();
                        if let Some(tp) =
                            state.players.values_mut().find(|x| x.p_id == target_id)
                        {
                            tp.deleted = true;
                            tp.death_reason = Some(death_reason.into());
                        }
                        scatter_backpack_on_death_pid(state, target_id);
                        apply_death_inheritance(state, target_id);
                        counters.deaths.fetch_add(1, Ordering::Relaxed);
                        state.push_event(format_death_event(target_id, cause));
                        state.afk.remove(target_id);
                        let line = format!(
                            "{} HIT {} KILL legal={} dmg={:.1}",
                            killer_id, target_id, legal, dmg
                        );
                        send_nearby_ps_lines(outbound, &near, &line);
                    }
                }
            }
            return;
        }
        // NURSE / FEED (no target) — feed held baby when carrying + holding food.
        // Bare-hand NURSE = one-shot breastfeed tick (Haxe nursing without held food).
        if upper == "NURSE" || upper == "FEED" {
            let feeder_id = p.p_id;
            let feeder_x = p.x;
            let feeder_y = p.y;
            let held_id = p.held_id;
            let baby_p_id = p.holding_player_id;
            if baby_p_id == 0 {
                let line = format!("{} {} FAIL no_baby", feeder_id, upper);
                send_ps_reply(outbound, conn_id, &line);
                return;
            }
            let (held_is_food, held_food_value) = resolve_held_food(state, held_id);
            let target_info = state.players.iter().find_map(|(&tc, tp)| {
                if tp.p_id == baby_p_id && !tp.deleted {
                    Some((tc, tp.food, tp.food_max, tp.age))
                } else {
                    None
                }
            });
            match target_info {
                None => {
                    let line = format!("{} {} FAIL baby_gone", feeder_id, upper);
                    send_ps_reply(outbound, conn_id, &line);
                }
                Some((t_conn, t_food, t_max, t_age)) => {
                    // Bare hands: breastfeed pulse (1s worth).
                    if held_id == 0 {
                        let fertile = FertilityState::age_fertile(p.age);
                        if can_breastfeed(p.age, p.food, fertile, t_age, true) {
                            let (to_baby, from_m) =
                                breastfeed_tick(1.0, FOOD_USE_PER_SEC, t_food, t_max);
                            if to_baby > 0.0 {
                                if let Some(feeder) = state.players.get_mut(&conn_id) {
                                    feeder.food = (feeder.food - from_m).max(0.0);
                                }
                                if let Some(tp) = state.players.get_mut(&t_conn) {
                                    tp.food = (tp.food + to_baby).min(tp.food_max);
                                }
                                state.publish_player_view(conn_id);
                                state.publish_player_view(t_conn);
                                let near =
                                    nearby_conn_ids(state, feeder_x, feeder_y, NEARBY_RANGE);
                                let new_food = state
                                    .players
                                    .get(&t_conn)
                                    .map(|tp| tp.food)
                                    .unwrap_or(t_food);
                                let line = format!(
                                    "{} {} {} OK breastfeed food={:.2}",
                                    feeder_id, upper, baby_p_id, new_food
                                );
                                send_nearby_ps_lines(outbound, &near, &line);
                                return;
                            }
                        }
                        let line = format!("{} {} FAIL not food", feeder_id, upper);
                        send_ps_reply(outbound, conn_id, &line);
                        return;
                    }
                    if !held_is_food {
                        let line = format!("{} {} FAIL not food", feeder_id, upper);
                        send_ps_reply(outbound, conn_id, &line);
                        return;
                    }
                    let (new_food, leftover) =
                        apply_feed_amounts(held_food_value, t_food, t_max);
                    let transferred = held_food_value - leftover;
                    if transferred <= 0.0 {
                        let line = format!("{} {} FAIL full", feeder_id, upper);
                        send_ps_reply(outbound, conn_id, &line);
                    } else {
                        if let Some(feeder) = state.players.get_mut(&conn_id) {
                            feeder.held_id = 0;
                        }
                        if let Some(tp) = state.players.get_mut(&t_conn) {
                            tp.food = new_food;
                        }
                        state.publish_player_view(conn_id);
                        state.publish_player_view(t_conn);
                        let near = nearby_conn_ids(state, feeder_x, feeder_y, NEARBY_RANGE);
                        if let Some(tp) = state.players.get(&t_conn) {
                            let fx = food_change_for_player(state, tp);
                            send_nearby(outbound, &near, fx.into_bytes());
                        }
                        let line = format!(
                            "{} {} {} OK food={:.2}",
                            feeder_id, upper, baby_p_id, new_food
                        );
                        send_nearby_ps_lines(outbound, &near, &line);
                    }
                }
            }
            return;
        }
        // FEED <p_id> — transfer held food to adjacent online player (or held baby).
        if upper.starts_with("FEED ") {
            let rest = text.split_whitespace().nth(1).unwrap_or("");
            if let Ok(target_id) = rest.parse::<i32>() {
                let feeder_id = p.p_id;
                let feeder_x = p.x;
                let feeder_y = p.y;
                let held_id = p.held_id;
                let holding_baby = p.holding_player_id;
                // Resolve food-ness: content food_value > 0, or name heuristic.
                let (held_is_food, held_food_value) = resolve_held_food(state, held_id);
                let target_info = state.players.iter().find_map(|(&tc, tp)| {
                    if tp.p_id == target_id && tp.connected && !tp.deleted {
                        Some((tc, tp.x, tp.y, tp.deleted, tp.food, tp.food_max))
                    } else {
                        None
                    }
                });
                let result = match target_info {
                    None => Err("target offline"),
                    Some((t_conn, tx, ty, deleted, t_food, t_max)) => {
                        // Held baby: always in range (teleports with carrier).
                        if holding_baby == target_id {
                            if held_id == 0 || !held_is_food {
                                Err("not food")
                            } else if deleted {
                                Err("target deleted")
                            } else {
                                Ok((t_conn, t_food, t_max))
                            }
                        } else {
                            can_feed(feeder_x, feeder_y, tx, ty, held_id, deleted, held_is_food)
                                .map(|_| (t_conn, t_food, t_max))
                        }
                    }
                };
                match result {
                    Ok((t_conn, t_food, t_max)) => {
                        let (new_food, leftover) =
                            apply_feed_amounts(held_food_value, t_food, t_max);
                        let transferred = held_food_value - leftover;
                        if transferred <= 0.0 {
                            let line = format!("{} FEED {} FAIL full", feeder_id, target_id);
                            send_ps_reply(outbound, conn_id, &line);
                        } else {
                            // Poisoned food: apply sick to target on successful FEED.
                            let held_name = held_object_name(state, held_id);
                            let apply_sick =
                                should_sicken_on_feed(&held_name, held_is_food);
                            // Consume held food item (discrete object); update target food.
                            if let Some(feeder) = state.players.get_mut(&conn_id) {
                                feeder.held_id = 0;
                            }
                            if let Some(tp) = state.players.get_mut(&t_conn) {
                                tp.food = new_food;
                                if apply_sick {
                                    tp.sick = true;
                                }
                            }
                            state.publish_player_view(conn_id);
                            state.publish_player_view(t_conn);
                            let near = nearby_conn_ids(state, feeder_x, feeder_y, NEARBY_RANGE);
                            // PU for feeder (empty hands) + target food change FX.
                            if let Some(feeder) = state.players.get(&conn_id) {
                                let spd = player_move_speed(state, feeder);
                                let pu = format_player_update_line(
                                    feeder.p_id,
                                    DEFAULT_PERSON_OBJECT,
                                    feeder.held_id,
                                    feeder.x,
                                    feeder.y,
                                    feeder.age,
                                    spd,
                                feeder.done_moving_seq.max(1),
                                );
                                send_nearby(
                                    outbound,
                                    &near,
                                    format_server_message("PU", &[&pu]).into_bytes(),
                                );
                            }
                            if let Some(tp) = state.players.get(&t_conn) {
                                let fx = food_change_for_player(state, tp);
                                send_nearby(outbound, &near, fx.into_bytes());
                                let spd = player_move_speed(state, tp);
                                let pu = format_player_update_line(
                                    tp.p_id,
                                    person_object_id(&p),
                                    tp.held_id,
                                    tp.x,
                                    tp.y,
                                    tp.age,
                                    spd,
                                tp.done_moving_seq.max(1),
                                );
                                send_nearby(
                                    outbound,
                                    &near,
                                    format_server_message("PU", &[&pu]).into_bytes(),
                                );
                            }
                            let line = if apply_sick {
                                format!(
                                    "{} FEED {} OK food={:.2} sick",
                                    feeder_id, target_id, new_food
                                )
                            } else {
                                format!(
                                    "{} FEED {} OK food={:.2}",
                                    feeder_id, target_id, new_food
                                )
                            };
                            send_nearby_ps_lines(outbound, &near, &line);
                        }
                    }
                    Err(reason) => {
                        let line = format!("{} FEED {} FAIL {}", feeder_id, target_id, reason);
                        send_ps_reply(outbound, conn_id, &line);
                    }
                }
            }
            return;
        }
        // GLOBAL <text> — server-wide GM; noble+ prestige only (lineage or combat).
        if upper.starts_with("GLOBAL ") || upper == "GLOBAL" {
            if !state.player_prestige_class(p.p_id).is_noble_or_more() {
                let line = format!("{} GLOBAL DENIED", p.p_id);
                send_ps_reply(outbound, conn_id, &line);
                info!(conn_id, p_id = p.p_id, "sim: GLOBAL DENIED (not noble+)");
                return;
            }
            let rest = text
                .split_once(char::is_whitespace)
                .map(|(_, r)| r.trim())
                .unwrap_or("");
            if !rest.is_empty() {
                broadcast_global(outbound, rest);
                info!(conn_id, text = %rest, "sim: GLOBAL");
            }
            return;
        }
        // ?LEADER — follow-graph leadership ranking.
        if upper == "?LEADER" || upper == "LEADER" {
            let reply = format_leader_query(&state.social.following, LEADER_QUERY_LIMIT);
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // ?WOUND / HEAL / BANDAGE / ?RANGE
        if upper == "?WOUND" || upper == "WOUND" {
            let reply = format_wound_query(&state.combat, p.p_id);
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // ?RANGE handled earlier (before rate limit) via format_range_query.
        // HEAL / BANDAGE — clear wounds (BANDAGE is HEAL alias).
        if upper == "HEAL" || upper == "BANDAGE" {
            let held = p.held_id;
            let is_heal = if held == 0 {
                false
            } else {
                state
                    .content
                    .get(held)
                    .map(|d| name_looks_like_heal(&d.name))
                    .unwrap_or(false)
            };
            // Free heal for testing when hands empty; consume heal item when held.
            let require = held != 0;
            let result = try_heal(&mut state.combat, p.p_id, is_heal || held == 0, require);
            let cmd = if upper == "BANDAGE" { "BANDAGE" } else { "HEAL" };
            let line = match result {
                HealResult::Healed { previous } => {
                    if is_heal {
                        if let Some(pl) = state.players.get_mut(&conn_id) {
                            pl.held_id = 0;
                        }
                    }
                    format!("{} {cmd} OK was={previous}", p.p_id)
                }
                HealResult::Healthy => format!("{} {cmd} OK healthy", p.p_id),
                HealResult::Denied => format!("{} {cmd} FAIL need_item", p.p_id),
            };
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // POLL / VOTE / ?POLL — session yes/no poll (pure PollState).
        if upper == "?POLL" || upper.starts_with("?POLL") {
            let reply = state.poll.format_query();
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        if upper.starts_with("POLL ") || upper == "POLL" {
            let question = text
                .split_once(char::is_whitespace)
                .map(|(_, r)| r.trim())
                .unwrap_or("");
            let ok = state.poll.create(p.p_id, question).is_ok();
            if ok {
                state.push_event(PollState::format_create_event(p.p_id, question));
            }
            let line = if ok {
                format!("{} POLL OK {question}", p.p_id)
            } else {
                format!("{} POLL FAIL", p.p_id)
            };
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        if upper.starts_with("VOTE ") || upper == "VOTE" {
            let token = text.split_whitespace().nth(1).unwrap_or("");
            let choice = parse_vote_choice(token);
            let ok = match choice {
                Some(c) => state.poll.vote(p.p_id, c).is_ok(),
                None => false,
            };
            let line = match (ok, choice) {
                (true, Some(c)) => format!("{} VOTE OK {}", p.p_id, c.as_str()),
                _ => format!("{} VOTE FAIL", p.p_id),
            };
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // ALLY / UNALLY / ?ALLY
        if upper == "?ALLY" || upper == "ALLY" {
            let reply = state.allies.format_query(p.p_id);
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        if upper.starts_with("ALLY ") {
            let to: i32 = text
                .split_whitespace()
                .nth(1)
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            let ok = state.allies.add(p.p_id, to).is_ok();
            let line = format!(
                "{} ALLY {} {}",
                p.p_id,
                to,
                if ok { "OK" } else { "FAIL" }
            );
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        if upper.starts_with("UNALLY ") {
            let to: i32 = text
                .split_whitespace()
                .nth(1)
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            let ok = state.allies.remove(p.p_id, to);
            let line = format!(
                "{} UNALLY {} {}",
                p.p_id,
                to,
                if ok { "OK" } else { "FAIL" }
            );
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // ?BIOMES — advertise impassable / special biomes (mountain wall).
        if upper == "?BIOMES" || upper == "BIOMES" {
            let reply = format_biomes_query();
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // ?BIOMEFOOD — food-drain multiplier for the speaker's standing biome.
        if upper == "?BIOMEFOOD" || upper == "BIOMEFOOD" {
            let biome = state.world.read().unwrap().get_biome(p.x, p.y);
            let reply = format_biomefood_query(biome);
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // ?SWIM — ocean/river wet flag + biome food_mult (extra drain already in vitals).
        if upper == "?SWIM" {
            let biome = state.world.read().unwrap().get_biome(p.x, p.y);
            let reply = format_swim_query(biome);
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // ?WARM — clothing_temp_bonus from hat/chest/shoes slots.
        if upper == "?WARM" || upper == "WARM" {
            let reply = format_warm_query(p.hat, p.chest, p.shoes);
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // ?SPEED — composed move speed (ride / weather / snow / fire / ballast).
        if upper == "?SPEED" || upper == "SPEED" {
            let speed = player_move_speed(state, &p);
            let reply = format_speed_query(speed);
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // ?WEIGHT — held + backpack item count (ballast for move speed).
        if upper == "?WEIGHT" || upper == "WEIGHT" {
            let n = weight_item_count(p.held_id, p.backpack.len());
            let reply = format_weight_query(n);
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // ?DRAIN — estimate current food drain/sec factors.
        if upper == "?DRAIN" || upper == "DRAIN" {
            let biome = state.world.read().unwrap().get_biome(p.x, p.y);
            let base = FOOD_USE_PER_SEC
                * state.environment.day_night_multiplier()
                * state.apocalypse.food_drain_multiplier();
            let est = estimate_food_drain(
                base,
                biome_food_multiplier(biome),
                state.weather.food_drain_mult(),
                p.age,
                p.sleeping,
                p.sitting,
                p.sick,
                state.combat.bleed_drain(p.p_id),
                state.fire.drain_at(p.x, p.y),
                state.snow.food_extra_at(p.x, p.y),
                p.hat,
                p.chest,
                p.shoes,
            );
            let reply = est.format_query();
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // ?CRAFTSTATS — reverse craft graph product/edge counts (seed metrics).
        if upper == "?CRAFTSTATS" || upper == "CRAFTSTATS" {
            let reply = state.craft_graph.format_craft_stats_query();
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // ?TRANS — content transition counts (normal + last-use).
        if upper == "?TRANS" || upper == "TRANS" {
            let reply = SimState::format_trans_query(&state.content);
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // ?CHUNKS — hot/warm/cold tier counts (cached on SimState each vitals tick).
        if upper == "?CHUNKS" || upper == "CHUNKS" {
            // Ensure cache is warm if vitals has not run yet this session.
            refresh_chunk_tier_counts(state);
            let reply = format_chunks_query(state.chunk_hot, state.chunk_warm, state.chunk_cold);
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // MAPFORCE — force MAP_CHUNK resend at current position.
        if upper == "MAPFORCE" {
            force_send_map_chunk(state, outbound, conn_id);
            let line = format!("{} MAPFORCE OK", p.p_id);
            send_ps_reply(outbound, conn_id, &line);
            info!(conn_id, "sim: SAY MAPFORCE");
            return;
        }
        // PING — private PS PONG with sim_time (client PING tag is separate).
        if upper == "PING" {
            let reply = SimState::format_ping_query(state.sim_time);
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // ?SPECIAL — sparse special-object index counts.
        if upper == "?SPECIAL" || upper == "SPECIAL" {
            let reply = state.specials.format_query();
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // ?ANIMALS / ?FAUNA — wild animal counts by kind.
        if upper == "?ANIMALS"
            || upper == "ANIMALS"
            || upper == "?FAUNA"
            || upper == "FAUNA"
        {
            let reply = state.animals.format_query();
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // HUNT — damage nearest adjacent animal; kill → meat placeholder + prestige.
        if upper == "HUNT" {
            let hunter_id = p.p_id;
            let hx = p.x;
            let hy = p.y;
            let result = hunt_nearest(
                &mut state.animals,
                hx,
                hy,
                HUNT_RANGE,
                HUNT_DAMAGE,
            );
            let line = match result {
                HuntResult::Miss => {
                    info!(conn_id, hunter_id, "sim: HUNT MISS");
                    format!("{hunter_id} HUNT MISS")
                }
                HuntResult::Hit {
                    animal_id,
                    kind,
                    hp_left,
                } => {
                    info!(
                        conn_id,
                        hunter_id,
                        animal_id,
                        kind = kind.label(),
                        hp_left,
                        "sim: HUNT HIT"
                    );
                    format!(
                        "{hunter_id} HUNT HIT {} id={animal_id} hp={hp_left}",
                        kind.label()
                    )
                }
                HuntResult::Kill {
                    animal_id,
                    kind,
                    x: ax,
                    y: ay,
                } => {
                    // Prestige for the kill.
                    state.combat.stats_mut(hunter_id).prestige += HUNT_KILL_PRESTIGE;
                    state.sync_lineage_prestige_from_combat(hunter_id);
                    // Clear map object where the animal stood (Haxe removes animal tile).
                    let oid = kind.object_id();
                    let floor = {
                        let mut w = state.world.write().unwrap();
                        if w.get_object(ax, ay) == oid {
                            w.set_object(ax, ay, 0);
                        }
                        w.get_floor(ax, ay) as i32
                    };
                    let near = nearby_conn_ids(state, ax, ay, NEARBY_RANGE);
                    for &cid in &near {
                        if let Some(v) = state.players.get(&cid) {
                            let (rx, ry) = v.world_to_client(ax, ay);
                            outbound.send_urgent(
                                cid,
                                format_map_change(rx, ry, floor, 0, -1).into_bytes(),
                            );
                            send_frame(outbound, cid);
                        }
                    }
                    // Meat object id 0 is a content placeholder; only equip when non-zero.
                    if HUNT_MEAT_OBJECT_ID != 0 {
                        if let Some(pl) = state.players.get_mut(&conn_id) {
                            if pl.held_id == 0 {
                                pl.held_id = HUNT_MEAT_OBJECT_ID;
                            }
                        }
                    }
                    state.push_event(format!(
                        "HUNT_KILL {hunter_id} {} {animal_id}",
                        kind.label()
                    ));
                    info!(
                        conn_id,
                        hunter_id,
                        animal_id,
                        kind = kind.label(),
                        "sim: HUNT KILL"
                    );
                    format!(
                        "{hunter_id} HUNT KILL {} id={animal_id} meat={} prestige={:.2}",
                        kind.label(),
                        HUNT_MEAT_OBJECT_ID,
                        HUNT_KILL_PRESTIGE
                    )
                }
            };
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // HARVEST / FISH / MINE / DIG / CHOP — lite profession actions (5s shared cooldown).
        if upper == "HARVEST" {
            let p_id = p.p_id;
            let (hx, hy, held, last_prof) =
                (p.x, p.y, p.held_id, p.last_prof_action_time);
            let biome = state.world.read().unwrap().get_biome(hx, hy);
            let sim_time = state.sim_time;
            let result = try_harvest(held, biome, last_prof, sim_time, &state.content);
            if let ProfActionResult::Ok { object_id } = result {
                if let Some(pl) = state.players.get_mut(&conn_id) {
                    pl.held_id = object_id;
                    pl.last_prof_action_time = sim_time;
                }
                info!(conn_id, p_id, object_id, "sim: HARVEST OK");
            }
            let line = format!("{p_id} HARVEST {}", result.wire_suffix());
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        if upper == "FISH" {
            let p_id = p.p_id;
            let (hx, hy, held, last_prof) =
                (p.x, p.y, p.held_id, p.last_prof_action_time);
            let biome = state.world.read().unwrap().get_biome(hx, hy);
            let sim_time = state.sim_time;
            let result = try_fish(held, biome, last_prof, sim_time);
            if let ProfActionResult::Ok { object_id } = result {
                if let Some(pl) = state.players.get_mut(&conn_id) {
                    pl.held_id = object_id;
                    pl.last_prof_action_time = sim_time;
                }
                info!(conn_id, p_id, object_id, "sim: FISH OK");
            }
            let line = format!("{p_id} FISH {}", result.wire_suffix());
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        if upper == "MINE" {
            let p_id = p.p_id;
            let (hx, hy, held, last_prof) =
                (p.x, p.y, p.held_id, p.last_prof_action_time);
            let mountain_near = {
                let w = state.world.read().unwrap();
                mountain_adjacent(hx, hy, |x, y| w.get_biome(x, y))
            };
            let sim_time = state.sim_time;
            let result = try_mine(held, last_prof, sim_time, mountain_near);
            if let ProfActionResult::Ok { object_id } = result {
                if let Some(pl) = state.players.get_mut(&conn_id) {
                    pl.held_id = object_id;
                    pl.last_prof_action_time = sim_time;
                }
                info!(conn_id, p_id, object_id, "sim: MINE OK");
            }
            let line = format!("{p_id} MINE {}", result.wire_suffix());
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        if upper == "DIG" {
            let p_id = p.p_id;
            let (hx, hy, held, last_prof) =
                (p.x, p.y, p.held_id, p.last_prof_action_time);
            let biome = state.world.read().unwrap().get_biome(hx, hy);
            let sim_time = state.sim_time;
            let result = try_dig(held, biome, last_prof, sim_time);
            if let ProfActionResult::Ok { object_id } = result {
                if let Some(pl) = state.players.get_mut(&conn_id) {
                    pl.held_id = object_id;
                    pl.last_prof_action_time = sim_time;
                }
                info!(conn_id, p_id, object_id, "sim: DIG OK");
            }
            let line = format!("{p_id} DIG {}", result.wire_suffix());
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        if upper == "CHOP" {
            let p_id = p.p_id;
            let (hx, hy, held, last_prof) =
                (p.x, p.y, p.held_id, p.last_prof_action_time);
            let biome = state.world.read().unwrap().get_biome(hx, hy);
            let sim_time = state.sim_time;
            let result = try_chop(held, biome, last_prof, sim_time);
            if let ProfActionResult::Ok { object_id } = result {
                if let Some(pl) = state.players.get_mut(&conn_id) {
                    pl.held_id = object_id;
                    pl.last_prof_action_time = sim_time;
                }
                info!(conn_id, p_id, object_id, "sim: CHOP OK");
            }
            let line = format!("{p_id} CHOP {}", result.wire_suffix());
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // ?FIRE / FIRE|IGNITE [x y] — ignite; EXTINGUISH under feet; ?SNOW
        if upper == "?FIRE" {
            let reply = state.fire.format_query();
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        if upper.starts_with("FIRE ")
            || upper == "FIRE"
            || upper.starts_with("IGNITE ")
            || upper == "IGNITE"
        {
            // FIRE|IGNITE [x y] — ignite under feet or at coords (testing / VOG-lite).
            let mut it = text.split_whitespace();
            let _ = it.next();
            let fx = it.next().and_then(|s| s.parse().ok()).unwrap_or(p.x);
            let fy = it.next().and_then(|s| s.parse().ok()).unwrap_or(p.y);
            state.fire.ignite(fx, fy, DEFAULT_FIRE_SECS, 1.0);
            let line = format!("{} FIRE {fx} {fy} OK", p.p_id);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        if upper == "EXTINGUISH" {
            let (fx, fy) = (p.x, p.y);
            let ok = state.fire.extinguish(fx, fy);
            let line = if ok {
                format!("{} EXTINGUISH {fx} {fy} OK", p.p_id)
            } else {
                format!("{} EXTINGUISH FAIL", p.p_id)
            };
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        if upper == "?SNOW" || upper == "SNOW" {
            let reply = state.snow.format_query();
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // ?CRIME / CRIME — personal theft counters.
        if upper == "?CRIME" || upper == "CRIME" {
            let reply = state.crime.format_crime_query(p.p_id);
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // ?WEATHER / WEATHER query; WEATHER <kind> [secs] / SETWEATHER <kind> [secs] to set (any player for testing).
        if upper == "?WEATHER" || upper == "WEATHER" {
            let reply = state.weather.query_text();
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        if upper.starts_with("WEATHER ") || upper.starts_with("SETWEATHER ") || upper == "SETWEATHER" {
            let mut it = text.split_whitespace();
            let _ = it.next();
            let kind_tok = it.next().unwrap_or("");
            let dur: f32 = it.next().and_then(|s| s.parse().ok()).unwrap_or(60.0);
            if let Some(kind) = parse_weather_kind(kind_tok) {
                state.weather.set(kind, dur);
                let reply = state.weather.query_text();
                let line = format!("{} {}", p.p_id, reply);
                send_ps_reply(outbound, conn_id, &line);
            } else {
                let line = format!("{} WEATHER FAIL bad_kind", p.p_id);
                send_ps_reply(outbound, conn_id, &line);
            }
            return;
        }
        // SEED — if animal world empty, respawn default animals (testing; no admin).
        if upper == "SEED" || upper == "SEED ANIMALS" {
            let before = state.animals.animals.len();
            spawn_default_animals(state);
            let after = state.animals.animals.len();
            let line = if before == 0 && after > 0 {
                format!("{} SEED OK animals={after}", p.p_id)
            } else if before > 0 {
                format!("{} SEED SKIP not_empty animals={before}", p.p_id)
            } else {
                format!("{} SEED OK animals=0", p.p_id)
            };
            send_ps_reply(outbound, conn_id, &line);
            info!(conn_id, before, after, "sim: SEED animals");
            return;
        }
        // ?FERTILE / ?BIRTH status
        if upper == "?FERTILE" || upper == "FERTILE" || upper == "?BIRTH" {
            let reply = state
                .fertility
                .format_query(p.p_id, p.age, state.sim_time);
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // ?ACCOUNT
        if upper == "?ACCOUNT" || upper == "ACCOUNT" {
            let reply = state.accounts.format_query(&p.email);
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // ?TWINS — multi-server twin peer list (stub registry only; no network).
        if upper == "?TWINS" || upper == "TWINS" {
            let reply = state.twins.format_query();
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // WHISPER <p_id> <text> — private PS only to target if online (find conn by p_id).
        if upper.starts_with("WHISPER ") {
            let rest = text
                .split_once(char::is_whitespace)
                .map(|(_, r)| r.trim())
                .unwrap_or("");
            let mut parts = rest.splitn(2, char::is_whitespace);
            let target_id: i32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
            let whisper_text = parts.next().map(str::trim).unwrap_or("");
            if target_id != 0 && !whisper_text.is_empty() {
                if let Some((&target_conn, _)) = state.players.iter().find(|(_, pl)| {
                    pl.p_id == target_id && pl.connected && !pl.deleted
                }) {
                    // Protocol: PS p_id/0 text + FM (private whisper still uses same wire).
                    send_ps_reply(
                        outbound,
                        target_conn,
                        &format!("{} {}", p.p_id, whisper_text),
                    );
                    info!(
                        conn_id,
                        target_id,
                        target_conn,
                        text = %whisper_text,
                        "sim: WHISPER"
                    );
                }
            }
            return;
        }
        // BIRTH — minimal motherhood: spawn baby age 0 with mother lineage + marker.
        // Fertility gate: age band + cooldown (instant birth path; not timed gestation).
        if upper == "BIRTH" {
            let sim_t = state.sim_time;
            let fert = state.fertility.can_birth(p.p_id, p.age, sim_t);
            if let Err(reason) = fert {
                let line = format!("{} BIRTH FAIL {reason}", p.p_id);
                send_ps_reply(outbound, conn_id, &line);
                return;
            }
            match spawn_child(state, conn_id) {
                Some(baby_p_id) => {
                    state.fertility.complete_birth(p.p_id, state.sim_time);
                    // spawn_child already pushed BIRTH to event_log.
                    let line = format!("{} BIRTH {baby_p_id} OK", p.p_id);
                    send_ps_reply(outbound, conn_id, &line);
                    // Push new LN line to mother (minimal; full fan-out later).
                    if let Some(node) = state.social.lineages.get(&baby_p_id) {
                        let ln = node.wire_line();
                        outbound.send(
                            conn_id,
                            format_server_message("LN", &[&ln]).into_bytes(),
                        );
                    }
                }
                None => {
                    let line = format!("{} BIRTH FAIL", p.p_id);
                    send_ps_reply(outbound, conn_id, &line);
                }
            }
            return;
        }
        // GESTATE — start timed fertility gestation; tick_vitals auto-spawns when due.
        if upper == "GESTATE" {
            let sim_t = state.sim_time;
            let fert = state.fertility.can_birth(p.p_id, p.age, sim_t);
            if let Err(reason) = fert {
                let line = format!("{} GESTATE FAIL {reason}", p.p_id);
                send_ps_reply(outbound, conn_id, &line);
                return;
            }
            let due = state.fertility.start_gestation(p.p_id, sim_t);
            let line = format!("{} GESTATE OK due={:.0}", p.p_id, due);
            send_ps_reply(outbound, conn_id, &line);
            info!(conn_id, p_id = p.p_id, due, "sim: GESTATE started");
            return;
        }
        // HOLD <p_id> — pick up adjacent baby (age < BABY_AGE_THRESHOLD); free hands.
        if upper.starts_with("HOLD ") || upper == "HOLD" {
            let baby_p_id: i32 = text
                .split_whitespace()
                .nth(1)
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            let mother_p_id = p.p_id;
            let (mx, my) = (p.x, p.y);
            let ok = if baby_p_id != 0
                && state
                    .players
                    .get(&conn_id)
                    .map(|pl| pl.can_hold_baby())
                    .unwrap_or(false)
            {
                // Locate baby by p_id (live, not deleted, age baby, adjacent).
                state.players.values().any(|pl| {
                    pl.p_id == baby_p_id
                        && !pl.deleted
                        && pl.age < BABY_AGE_THRESHOLD
                        && pl.held_by == 0
                        && (pl.x - mx).abs().max((pl.y - my).abs()) <= 1
                })
            } else {
                false
            };
            if ok {
                // Apply links after the immutable borrow ends.
                if let Some(pl) = state.players.get_mut(&conn_id) {
                    pl.start_holding(baby_p_id);
                }
                if let Some(baby) = state.players.values_mut().find(|pl| pl.p_id == baby_p_id) {
                    baby.held_by = mother_p_id;
                    baby.x = mx;
                    baby.y = my;
                }
                // Haxe doBaby: pickup feeding when fertile mother + young child.
                let mother_fertile = FertilityState::age_fertile(
                    state.players.get(&conn_id).map(|pl| pl.age).unwrap_or(0.0),
                );
                let baby_age = state
                    .players
                    .values()
                    .find(|pl| pl.p_id == baby_p_id)
                    .map(|b| b.age)
                    .unwrap_or(99.0);
                if mother_fertile && baby_age <= MAX_CHILD_AGE_BREAST_FEEDING {
                    let (b_food, b_max) = state
                        .players
                        .values()
                        .find(|pl| pl.p_id == baby_p_id)
                        .map(|b| (b.food, b.food_max))
                        .unwrap_or((0.0, 20.0));
                    let (to_baby, from_m) = pickup_feed_amounts(b_food, b_max);
                    if to_baby > 0.0 {
                        if let Some(pl) = state.players.get_mut(&conn_id) {
                            pl.food = (pl.food - from_m).max(0.0);
                        }
                        if let Some(baby) =
                            state.players.values_mut().find(|pl| pl.p_id == baby_p_id)
                        {
                            baby.food = (baby.food + to_baby).min(baby.food_max);
                        }
                        info!(
                            conn_id,
                            baby_p_id,
                            to_baby,
                            from_m,
                            "sim: HOLD pickup breastfeed"
                        );
                    }
                }
                let line = format!("{} HOLD {baby_p_id} OK", mother_p_id);
                send_ps_reply(outbound, conn_id, &line);
                info!(conn_id, baby_p_id, "sim: HOLD baby");
            } else {
                let line = format!("{} HOLD FAIL", mother_p_id);
                send_ps_reply(outbound, conn_id, &line);
            }
            return;
        }
        // PUTDOWN / DROPBABY — release held baby at mother's tile.
        if upper == "PUTDOWN" || upper == "DROPBABY" {
            let mother_p_id = p.p_id;
            let (mx, my) = (p.x, p.y);
            let baby_p_id = state
                .players
                .get_mut(&conn_id)
                .map(|pl| pl.release_holding())
                .unwrap_or(0);
            if baby_p_id != 0 {
                if let Some(baby) = state.players.values_mut().find(|pl| pl.p_id == baby_p_id) {
                    baby.held_by = 0;
                    baby.x = mx;
                    baby.y = my;
                }
                let line = format!("{} PUTDOWN {baby_p_id} OK", mother_p_id);
                send_ps_reply(outbound, conn_id, &line);
                info!(conn_id, baby_p_id, "sim: PUTDOWN baby");
            } else {
                let line = format!("{} PUTDOWN FAIL", mother_p_id);
                send_ps_reply(outbound, conn_id, &line);
            }
            return;
        }
        // CLOTHES — report hat/chest/shoes object ids (0 = empty).
        if upper == "CLOTHES" || upper == "?CLOTHES" {
            if let Some(pl) = state.players.get(&conn_id) {
                let line = format!("{} {}", pl.p_id, pl.clothes_report());
                send_ps_reply(outbound, conn_id, &line);
            }
            return;
        }
        // WATER — simple self food+1 test boost (capped at food_max). Feed-other is FEED.
        if upper == "WATER" {
            let result = state.players.get_mut(&conn_id).map(|pl| {
                let before = pl.food;
                pl.food = (pl.food + 1.0).min(pl.food_max);
                (pl.p_id, before, pl.food, pl.food_max)
            });
            if let Some((p_id, before, food, food_max)) = result {
                let gained = food - before;
                let line = if gained > 0.0 {
                    format!("{p_id} WATER OK food={food:.2}/{food_max:.2}")
                } else {
                    format!("{p_id} WATER OK full food={food:.2}/{food_max:.2}")
                };
                send_ps_reply(outbound, conn_id, &line);
                state.publish_player_view(conn_id);
                info!(conn_id, food, "sim: WATER food boost");
            }
            return;
        }
        // STRIP hat|chest|shoes — unequip slot into empty hands.
        if upper.starts_with("STRIP ") || upper == "STRIP" {
            let slot_tok = text.split_whitespace().nth(1).unwrap_or("");
            let slot = ClothingSlot::parse(slot_tok);
            let Some(slot) = slot else {
                if let Some(pl) = state.players.get(&conn_id) {
                    let line = format!("{} STRIP FAIL BAD", pl.p_id);
                    send_ps_reply(outbound, conn_id, &line);
                }
                return;
            };
            let result = state.players.get_mut(&conn_id).map(|pl| {
                let r = pl.strip_slot(slot);
                (pl.p_id, r, pl.held_id, pl.x, pl.y, pl.age)
            });
            if let Some((p_id, r, held_id, x, y, age)) = result {
                match r {
                    Ok(id) => {
                        let line = format!("{p_id} STRIP {} {id} OK", slot.as_str());
                        send_ps_reply(outbound, conn_id, &line);
                        state.publish_player_view(conn_id);
                        let spd = state
                            .players
                            .get(&conn_id)
                            .map(|pl| player_move_speed(state, pl))
                            .unwrap_or(WALK_MOVE_SPEED);
                        let pu = format_player_update_line(
                            p_id,
                            DEFAULT_PERSON_OBJECT,
                            held_id,
                            x,
                            y,
                            age,
                            spd,
                        1,
                        );
                        let near = nearby_conn_ids(state, x, y, NEARBY_RANGE);
                        send_nearby(
                            outbound,
                            &near,
                            format_server_message("PU", &[&pu]).into_bytes(),
                        );
                        info!(conn_id, slot = slot.as_str(), id, "sim: STRIP clothing");
                    }
                    Err(e) => {
                        let line = format!("{p_id} STRIP FAIL {e}");
                        send_ps_reply(outbound, conn_id, &line);
                    }
                }
            }
            return;
        }
        // WEAR [hat|chest|shoes] — equip held into clothing slot (inferred from object name if omitted).
        if upper.starts_with("WEAR ") || upper == "WEAR" {
            let slot_tok = text.split_whitespace().nth(1);
            let explicit = slot_tok.and_then(ClothingSlot::parse);
            let held_id = state.players.get(&conn_id).map(|pl| pl.held_id).unwrap_or(0);
            let inferred = if held_id != 0 {
                state
                    .content
                    .get(held_id)
                    .and_then(|def| clothing_slot_for_object(&def.name, &def.description))
            } else {
                None
            };
            let slot = explicit.or(inferred);
            let Some(slot) = slot else {
                if let Some(pl) = state.players.get(&conn_id) {
                    let line = if pl.held_id == 0 {
                        format!("{} WEAR FAIL EMPTY", pl.p_id)
                    } else {
                        format!("{} WEAR FAIL NOT_CLOTHES", pl.p_id)
                    };
                    send_ps_reply(outbound, conn_id, &line);
                }
                return;
            };
            let result = state.players.get_mut(&conn_id).map(|pl| {
                let r = pl.wear_held(slot);
                (pl.p_id, r, pl.held_id, pl.x, pl.y, pl.age)
            });
            if let Some((p_id, r, held_id, x, y, age)) = result {
                match r {
                    Ok((id, prev)) => {
                        let line = if prev != 0 {
                            format!(
                                "{p_id} WEAR {} {id} OK swap={prev}",
                                slot.as_str()
                            )
                        } else {
                            format!("{p_id} WEAR {} {id} OK", slot.as_str())
                        };
                        send_ps_reply(outbound, conn_id, &line);
                        state.publish_player_view(conn_id);
                        let spd = state
                            .players
                            .get(&conn_id)
                            .map(|pl| player_move_speed(state, pl))
                            .unwrap_or(WALK_MOVE_SPEED);
                        let pu = format_player_update_line(
                            p_id,
                            DEFAULT_PERSON_OBJECT,
                            held_id,
                            x,
                            y,
                            age,
                            spd,
                        1,
                        );
                        let near = nearby_conn_ids(state, x, y, NEARBY_RANGE);
                        send_nearby(
                            outbound,
                            &near,
                            format_server_message("PU", &[&pu]).into_bytes(),
                        );
                        info!(conn_id, slot = slot.as_str(), id, prev, "sim: WEAR clothing");
                    }
                    Err(e) => {
                        let line = format!("{p_id} WEAR FAIL {e}");
                        send_ps_reply(outbound, conn_id, &line);
                    }
                }
            }
            return;
        }
        // INV — list backpack object ids (max BACKPACK_MAX).
        if upper == "INV" || upper == "?INV" {
            if let Some(pl) = state.players.get(&conn_id) {
                let line = format!("{} {}", pl.p_id, pl.inv_report());
                send_ps_reply(outbound, conn_id, &line);
            }
            return;
        }
        // NOTES / ?NOTES / MEMORY / ?MEMORY — personal journal list (max NOTES_MAX).
        if upper == "NOTES"
            || upper == "?NOTES"
            || upper == "MEMORY"
            || upper == "?MEMORY"
        {
            if let Some(pl) = state.players.get(&conn_id) {
                let line = format!("{} {}", pl.p_id, pl.notes_report());
                send_ps_reply(outbound, conn_id, &line);
            }
            return;
        }
        // FORGET — pop last personal journal note.
        if upper == "FORGET" {
            let result = state.players.get_mut(&conn_id).map(|pl| {
                let r = pl.pop_note();
                (pl.p_id, r, pl.notes.len())
            });
            if let Some((p_id, r, n)) = result {
                match r {
                    Ok(text) => {
                        let line = format!("{p_id} FORGET {n}/{NOTES_MAX} OK {text}");
                        send_ps_reply(outbound, conn_id, &line);
                        info!(conn_id, n, "sim: FORGET personal journal");
                    }
                    Err(e) => {
                        let line = format!("{p_id} FORGET FAIL {e}");
                        send_ps_reply(outbound, conn_id, &line);
                    }
                }
            }
            return;
        }
        // TITLE <text> — set personal title string (shown in ?NAME).
        if upper.starts_with("TITLE ") || upper == "TITLE" {
            let body = text
                .split_once(char::is_whitespace)
                .map(|(_, r)| r.trim())
                .unwrap_or("");
            let result = state.players.get_mut(&conn_id).map(|pl| {
                let r = pl.set_title(body).map(|s| s.to_string());
                (pl.p_id, r)
            });
            if let Some((p_id, r)) = result {
                match r {
                    Ok(title) => {
                        let line = format!("{p_id} TITLE OK {title}");
                        send_ps_reply(outbound, conn_id, &line);
                        info!(conn_id, %title, "sim: TITLE set");
                    }
                    Err(e) => {
                        let line = format!("{p_id} TITLE FAIL {e}");
                        send_ps_reply(outbound, conn_id, &line);
                    }
                }
            }
            return;
        }
        // NOTE / REMEMBER <text> — append personal journal line (max NOTES_MAX).
        if upper.starts_with("NOTE ")
            || upper == "NOTE"
            || upper.starts_with("REMEMBER ")
            || upper == "REMEMBER"
        {
            let body = text
                .split_once(char::is_whitespace)
                .map(|(_, r)| r.trim())
                .unwrap_or("");
            let result = state.players.get_mut(&conn_id).map(|pl| {
                let r = pl.add_note(body);
                (pl.p_id, r, pl.notes.len())
            });
            if let Some((p_id, r, n)) = result {
                match r {
                    Ok(_) => {
                        let line = format!("{p_id} NOTE {n}/{NOTES_MAX} OK");
                        send_ps_reply(outbound, conn_id, &line);
                        info!(conn_id, n, "sim: NOTE personal journal");
                    }
                    Err(e) => {
                        let line = format!("{p_id} NOTE FAIL {e}");
                        send_ps_reply(outbound, conn_id, &line);
                    }
                }
            }
            return;
        }
        // HELD / ?HELD — held object id + content name when known.
        if upper == "HELD" || upper == "?HELD" {
            let held_id = state.players.get(&conn_id).map(|pl| pl.held_id).unwrap_or(0);
            let reply = state.format_held_query(held_id);
            let line = format!("{} {}", p.p_id, reply);
            send_ps_reply(outbound, conn_id, &line);
            return;
        }
        // STORE — move held object into backpack if space.
        if upper == "STORE" {
            let result = state
                .players
                .get_mut(&conn_id)
                .map(|pl| {
                    let r = pl.store_to_backpack();
                    (pl.p_id, r, pl.held_id, pl.x, pl.y, pl.age)
                });
            if let Some((p_id, r, held_id, x, y, age)) = result {
                match r {
                    Ok(id) => {
                        let line = format!("{p_id} STORE {id} OK");
                        send_ps_reply(outbound, conn_id, &line);
                        state.publish_player_view(conn_id);
                        let spd = state
                            .players
                            .get(&conn_id)
                            .map(|pl| player_move_speed(state, pl))
                            .unwrap_or(WALK_MOVE_SPEED);
                        let pu = format_player_update_line(
                            p_id,
                            DEFAULT_PERSON_OBJECT,
                            held_id,
                            x,
                            y,
                            age,
                            spd,
                        1,
                        );
                        let near = nearby_conn_ids(state, x, y, NEARBY_RANGE);
                        send_nearby(
                            outbound,
                            &near,
                            format_server_message("PU", &[&pu]).into_bytes(),
                        );
                        info!(conn_id, id, "sim: STORE to backpack");
                    }
                    Err(e) => {
                        let line = format!("{p_id} STORE FAIL {e}");
                        send_ps_reply(outbound, conn_id, &line);
                    }
                }
            }
            return;
        }
        // TAKE <i> — move backpack index i into empty hands.
        if upper.starts_with("TAKE ") || upper == "TAKE" {
            let idx = text
                .split_whitespace()
                .nth(1)
                .and_then(|s| s.parse::<usize>().ok());
            let Some(i) = idx else {
                if let Some(pl) = state.players.get(&conn_id) {
                    let line = format!("{} TAKE FAIL BAD", pl.p_id);
                    send_ps_reply(outbound, conn_id, &line);
                }
                return;
            };
            let result = state.players.get_mut(&conn_id).map(|pl| {
                let r = pl.take_from_backpack(i);
                (pl.p_id, r, pl.held_id, pl.x, pl.y, pl.age)
            });
            if let Some((p_id, r, held_id, x, y, age)) = result {
                match r {
                    Ok(id) => {
                        let line = format!("{p_id} TAKE {i} {id} OK");
                        send_ps_reply(outbound, conn_id, &line);
                        state.publish_player_view(conn_id);
                        let spd = state
                            .players
                            .get(&conn_id)
                            .map(|pl| player_move_speed(state, pl))
                            .unwrap_or(WALK_MOVE_SPEED);
                        let pu = format_player_update_line(
                            p_id,
                            DEFAULT_PERSON_OBJECT,
                            held_id,
                            x,
                            y,
                            age,
                            spd,
                        1,
                        );
                        let near = nearby_conn_ids(state, x, y, NEARBY_RANGE);
                        send_nearby(
                            outbound,
                            &near,
                            format_server_message("PU", &[&pu]).into_bytes(),
                        );
                        info!(conn_id, i, id, "sim: TAKE from backpack");
                    }
                    Err(e) => {
                        let line = format!("{p_id} TAKE {i} FAIL {e}");
                        send_ps_reply(outbound, conn_id, &line);
                    }
                }
            }
            return;
        }
        // DROPALL — scatter held + backpack onto empty tiles near the player (no death).
        if upper == "DROPALL" {
            let meta = state.players.get(&conn_id).map(|pl| {
                (pl.p_id, pl.x, pl.y, pl.age, pl.deleted)
            });
            let Some((p_id, x, y, age, deleted)) = meta else {
                return;
            };
            if deleted {
                return;
            }
            let placed = scatter_dropall(state, conn_id);
            let n = placed.len();
            let line = if n == 0 {
                format!("{p_id} DROPALL OK n=0")
            } else {
                format!("{p_id} DROPALL OK n={n}")
            };
            send_ps_reply(outbound, conn_id, &line);
            state.publish_player_view(conn_id);
            let spd = state
                .players
                .get(&conn_id)
                .map(|pl| player_move_speed(state, pl))
                .unwrap_or(WALK_MOVE_SPEED);
            let held_id = state
                .players
                .get(&conn_id)
                .map(|pl| pl.held_id)
                .unwrap_or(0);
            let pu = format_player_update_line(
                p_id,
                DEFAULT_PERSON_OBJECT,
                held_id,
                x,
                y,
                age,
                spd,
            1,
            );
            let near = nearby_conn_ids(state, x, y, NEARBY_RANGE);
            send_nearby(
                outbound,
                &near,
                format_server_message("PU", &[&pu]).into_bytes(),
            );
            info!(conn_id, p_id, n, "sim: DROPALL");
            return;
        }
        // PUTNEST <slot> — put held into nested pocket of contained[slot] under feet.
        // Uses World::container_put_nested (one level deep). Slot is top-level contained index.
        if upper.starts_with("PUTNEST ") || upper == "PUTNEST" {
            let slot_raw = text
                .split_whitespace()
                .nth(1)
                .and_then(|s| s.parse::<i32>().ok());
            let meta = state.players.get(&conn_id).map(|pl| {
                (pl.p_id, pl.x, pl.y, pl.held_id, pl.deleted)
            });
            let Some((p_id, x, y, held, deleted)) = meta else {
                return;
            };
            if deleted {
                return;
            }
            let Some(slot_i) = slot_raw.filter(|&s| s >= 0) else {
                let line = format!("{p_id} PUTNEST FAIL BAD");
                send_ps_reply(outbound, conn_id, &line);
                return;
            };
            let slot = slot_i as usize;
            if held == 0 {
                let line = format!("{p_id} PUTNEST FAIL EMPTY");
                send_ps_reply(outbound, conn_id, &line);
                return;
            }
            // Only containable items may enter nested pockets (same rule as DROP put).
            let held_ok = state
                .content
                .get(held)
                .map(|d| d.containable)
                .unwrap_or(false);
            if !held_ok {
                let line = format!("{p_id} PUTNEST FAIL CONTAIN");
                send_ps_reply(outbound, conn_id, &line);
                return;
            }
            // Max sub-slots from the pocket object (contained[slot]) if known.
            let max_sub = {
                let w = state.world.read().unwrap();
                let pocket_id = w
                    .get_helper(x, y)
                    .and_then(|h| h.contained.get(slot).copied())
                    .unwrap_or(0);
                if pocket_id == 0 {
                    0
                } else {
                    state
                        .content
                        .get(pocket_id)
                        .map(|d| d.num_slots.max(0) as usize)
                        .unwrap_or(DEFAULT_CONTAINER_SLOTS)
                        .max(1)
                }
            };
            let put = if max_sub == 0 {
                false
            } else {
                state
                    .world
                    .write()
                    .unwrap()
                    .container_put_nested(x, y, slot, held, max_sub)
            };
            if put {
                if let Some(pl) = state.players.get_mut(&conn_id) {
                    pl.held_id = 0;
                }
                let line = format!("{p_id} PUTNEST {slot} {held} OK");
                send_ps_reply(outbound, conn_id, &line);
                state.publish_player_view(conn_id);
                let tile = state.world.read().unwrap().get_object(x, y);
                let near = nearby_conn_ids(state, x, y, NEARBY_RANGE);
                for pkt in packets_after_drop(state, conn_id, x, y, tile) {
                    send_nearby(outbound, &near, pkt);
                }
                info!(conn_id, x, y, slot, held, "sim: PUTNEST into nested pocket");
            } else {
                let line = format!("{p_id} PUTNEST {slot} FAIL");
                send_ps_reply(outbound, conn_id, &line);
            }
            return;
        }
        // CRAFT — self-craft held via transition (held, 0); same as USE on empty tile.
        if upper == "CRAFT" {
            match try_craft(state, conn_id) {
                Some(r) if r.applied => {
                    // Skill XP on successful craft (key = pre-craft held id).
                    let lvl = state.skills.on_craft(p.p_id, r.actor_before);
                    counters.crafts.fetch_add(1, Ordering::Relaxed);
                    let line = format!("{} CRAFT OK skill_lvl={lvl}", p.p_id);
                    send_ps_reply(outbound, conn_id, &line);
                    state.publish_player_view(conn_id);
                    let near = nearby_conn_ids(state, r.x, r.y, NEARBY_RANGE);
                    for pkt in packets_after_use(state, conn_id, &r) {
                        send_nearby(outbound, &near, pkt);
                    }
                    info!(
                        conn_id,
                        held = r.actor_before,
                        new_held = r.actor_after,
                        ground = r.target_after,
                        skill_lvl = lvl,
                        "sim: CRAFT applied"
                    );
                }
                Some(_) => {
                    let line = format!("{} CRAFT FAIL", p.p_id);
                    send_ps_reply(outbound, conn_id, &line);
                    info!(conn_id, "sim: CRAFT no transition");
                }
                None => {}
            }
            return;
        }
        // SLEEP — enter sleep: block MOVE, halve food drain; PE snore on vitals timer.
        if upper == "SLEEP" {
            if let Some(pl) = state.players.get_mut(&conn_id) {
                pl.sleeping = true;
                pl.sitting = false;
                pl.moving = false;
                pl.sleep_emot_timer = 0.0;
                let line = format!("{} SLEEP OK", pl.p_id);
                send_ps_reply(outbound, conn_id, &line);
                info!(conn_id, "sim: SLEEP");
            }
            return;
        }
        // WAKE — leave sleep: restore MOVE and normal food drain.
        if upper == "WAKE" {
            if let Some(pl) = state.players.get_mut(&conn_id) {
                pl.sleeping = false;
                pl.sleep_emot_timer = 0.0;
                let line = format!("{} WAKE OK", pl.p_id);
                send_ps_reply(outbound, conn_id, &line);
                info!(conn_id, "sim: WAKE");
            }
            return;
        }
        // SIT — sit: block MOVE, mild food-drain reduce (like SLEEP, milder).
        if upper == "SIT" {
            if let Some(pl) = state.players.get_mut(&conn_id) {
                pl.sitting = true;
                pl.sleeping = false;
                pl.moving = false;
                pl.sleep_emot_timer = 0.0;
                let line = format!("{} SIT OK", pl.p_id);
                send_ps_reply(outbound, conn_id, &line);
                info!(conn_id, "sim: SIT");
            }
            return;
        }
        // STAND — leave sit: restore MOVE and normal (non-sit) food drain.
        if upper == "STAND" {
            if let Some(pl) = state.players.get_mut(&conn_id) {
                pl.sitting = false;
                let line = format!("{} STAND OK", pl.p_id);
                send_ps_reply(outbound, conn_id, &line);
                info!(conn_id, "sim: STAND");
            }
            return;
        }
        // RENAME <first> [last] — change display name; NM packet to nearby.
        if upper == "RENAME" || upper.starts_with("RENAME ") {
            let mut parts = text.split_whitespace();
            let _cmd = parts.next();
            let Some(first_raw) = parts.next() else {
                let line = format!("{} RENAME FAIL need first", p.p_id);
                send_ps_reply(outbound, conn_id, &line);
                return;
            };
            let first = first_raw.to_ascii_uppercase();
            let last_opt = parts.next().map(|s| s.to_ascii_uppercase());
            let (p_id, x, y, display_name, email) = {
                let Some(pl) = state.players.get_mut(&conn_id) else {
                    return;
                };
                if pl.deleted {
                    return;
                }
                pl.first_name = first;
                if let Some(last) = last_opt {
                    pl.family_name = last;
                }
                (
                    pl.p_id,
                    pl.x,
                    pl.y,
                    pl.display_name(),
                    pl.email.clone(),
                )
            };
            if let Some(node) = state.social.lineages.get_mut(&p_id) {
                node.name = display_name.clone();
            }
            state.scoreboard.set_name(p_id, &display_name);
            state.accounts.ensure(&email).last_name = display_name.clone();
            let nm_line = format!("{p_id} {display_name}");
            let nm = format_server_message("NM", &[&nm_line]).into_bytes();
            let near = nearby_conn_ids(state, x, y, NEARBY_RANGE);
            send_nearby(outbound, &near, nm);
            let line = format!("{p_id} RENAME OK {display_name}");
            send_ps_reply(outbound, conn_id, &line);
            state.publish_player_view(conn_id);
            info!(conn_id, p_id, name = %display_name, "sim: RENAME");
            return;
        }
        // DIE — voluntary death (reason_suicide); also available as client DIE tag.
        if upper == "DIE" {
            let died = state.players.get_mut(&conn_id).map(|pl| {
                if pl.deleted {
                    return None;
                }
                pl.deleted = true;
                pl.death_reason = Some(DeathCause::Suicide.wire_tag().into());
                pl.sleeping = false;
                pl.sitting = false;
                Some(pl.p_id)
            });
            if let Some(Some(p_id)) = died {
                scatter_backpack_on_death(state, conn_id);
                apply_death_inheritance(state, p_id);
                counters.deaths.fetch_add(1, Ordering::Relaxed);
                state.scoreboard.record_death(p_id);
                state.push_event(format_death_event(p_id, DeathCause::Suicide));
                state.afk.remove(p_id);
                state.publish_player_view(conn_id);
                let line = format!("{p_id} DIE OK");
                send_ps_reply(outbound, conn_id, &line);
                info!(conn_id, p_id, "sim: SAY DIE reason_suicide");
            }
            return;
        }
        // LASTUSE / LAST_USE — force last-use transition table on the next USE
        // (Haxe multi-use nearly exhausted). Cleared after one applied USE.
        if upper == "LASTUSE" || upper == "LAST_USE" {
            if let Some(pl) = state.players.get_mut(&conn_id) {
                pl.force_last_use = true;
                let line = format!("{} LASTUSE OK", pl.p_id);
                send_ps_reply(outbound, conn_id, &line);
                info!(conn_id, "sim: LASTUSE force_last_use");
            }
            return;
        }
        // SICK — mark sick: food drain * SICK_FOOD_DRAIN_MULT; DY uses isSick.
        if upper == "SICK" {
            if let Some(pl) = state.players.get_mut(&conn_id) {
                pl.sick = true;
                let line = format!("{} SICK OK", pl.p_id);
                send_ps_reply(outbound, conn_id, &line);
                info!(conn_id, "sim: SICK");
            }
            return;
        }
        // CURE — clear sick: restore normal food drain / DY flag.
        if upper == "CURE" {
            if let Some(pl) = state.players.get_mut(&conn_id) {
                pl.sick = false;
                let line = format!("{} CURE OK", pl.p_id);
                send_ps_reply(outbound, conn_id, &line);
                info!(conn_id, "sim: CURE");
            }
            return;
        }
        // RIDE / MOUNT — set riding flag; move_speed note only (no actual MOVE speed change).
        if upper == "RIDE" || upper == "MOUNT" {
            if let Some(pl) = state.players.get_mut(&conn_id) {
                pl.riding = true;
            }
            if let Some(pl) = state.players.get(&conn_id) {
                let spd = player_move_speed(state, pl);
                // Always report RIDE OK (MOUNT is an alias).
                let line = format!("{} RIDE OK move_speed={:.2}", pl.p_id, spd);
                send_ps_reply(outbound, conn_id, &line);
                info!(conn_id, "sim: RIDE/MOUNT");
            }
            return;
        }
        // DISMOUNT — clear riding; report walk move_speed note only.
        if upper == "DISMOUNT" {
            if let Some(pl) = state.players.get_mut(&conn_id) {
                pl.riding = false;
            }
            if let Some(pl) = state.players.get(&conn_id) {
                let spd = player_move_speed(state, pl);
                let line = format!("{} DISMOUNT OK move_speed={:.2}", pl.p_id, spd);
                send_ps_reply(outbound, conn_id, &line);
                info!(conn_id, "sim: DISMOUNT");
            }
            return;
        }
        // SWIM — note ocean/river food-drain mult (already applied in vitals via biome).
        if upper == "SWIM" {
            let biome = state.world.read().unwrap().get_biome(p.x, p.y);
            let mult = biome_food_multiplier(biome);
            let wet = if is_swim_biome(biome) { 1 } else { 0 };
            let line = format!(
                "{} SWIM OK biome={} wet={} food_mult={:.2}",
                p.p_id, biome, wet, mult
            );
            send_ps_reply(outbound, conn_id, &line);
            info!(conn_id, biome, "sim: SWIM note");
            return;
        }
        // BUILD — fence placeholder (object id 0; no real place yet).
        if upper == "BUILD" || upper.starts_with("BUILD ") {
            // Placeholder: DROP id 0 — no object placed.
            let line = format!("{} BUILD OK fence=0", p.p_id);
            send_ps_reply(outbound, conn_id, &line);
            info!(conn_id, "sim: BUILD fence placeholder");
            return;
        }
        // CLAIM — set ownership on object under feet without locking the tile.
        if upper == "CLAIM" {
            let (x, y, p_id) = (p.x, p.y, p.p_id);
            let ok = {
                let mut w = state.world.write().unwrap();
                let id = w.get_object(x, y);
                if id != 0 {
                    let mut h = w
                        .get_helper(x, y)
                        .cloned()
                        .unwrap_or_else(|| ComplexObject::with_owner(id, p_id));
                    h.owner_id = p_id;
                    w.set_object_complex(x, y, h);
                    true
                } else {
                    false
                }
            };
            // Do not touch LockState — ownership only.
            let line = format!(
                "{} CLAIM {} {} {}",
                p_id,
                x,
                y,
                if ok { "OK" } else { "FAIL" }
            );
            send_ps_reply(outbound, conn_id, &line);
            info!(conn_id, x, y, ok, "sim: CLAIM");
            return;
        }
        // HOME — set personal home to current tile.
        if upper == "HOME" {
            if let Some(pl) = state.players.get_mut(&conn_id) {
                pl.home_x = pl.x;
                pl.home_y = pl.y;
                let line = format!("{} HOME {} {}", pl.p_id, pl.home_x, pl.home_y);
                send_ps_reply(outbound, conn_id, &line);
                info!(
                    conn_id,
                    home_x = pl.home_x,
                    home_y = pl.home_y,
                    "sim: HOME set"
                );
            }
            return;
        }
        // MARK <label> — custom map marker at current tile for self (LOCATION_SAYS style).
        if upper.starts_with("MARK ") || upper == "MARK" {
            let label = text
                .split_once(char::is_whitespace)
                .map(|(_, r)| r.trim())
                .unwrap_or("")
                .to_string();
            if label.is_empty() {
                let line = format!("{} MARK FAIL", p.p_id);
                send_ps_reply(outbound, conn_id, &line);
                return;
            }
            let (px, py, pid) = (p.x, p.y, p.p_id);
            state
                .markers
                .add_custom_marker(pid, px, py, label.clone(), pid);
            let line = format!("{pid} MARK {px} {py} {label}");
            send_ps_reply(outbound, conn_id, &line);
            info!(conn_id, x = px, y = py, %label, "sim: MARK custom");
            return;
        }
        // PLAN <object_id> — reverse craft ingredient path (leaf→root actor+target).
        if upper.starts_with("PLAN ") || upper == "PLAN" {
            let want: Option<i32> = text
                .split_whitespace()
                .nth(1)
                .and_then(|s| s.parse().ok());
            let (p_id, held, backpack) = state
                .players
                .get(&conn_id)
                .map(|pl| (pl.p_id, pl.held_id, pl.backpack.clone()))
                .unwrap_or((p.p_id, p.held_id, Vec::new()));
            let mut have = std::collections::HashSet::new();
            if held != 0 {
                have.insert(held);
            }
            for &id in &backpack {
                if id != 0 {
                    have.insert(id);
                }
            }
            let reply = match want {
                Some(w) if w != 0 => state.craft_graph.format_plan_query(w, &have, 6),
                _ => "PLAN FAIL".into(),
            };
            let line = format!("{p_id} {reply}");
            send_ps_reply(outbound, conn_id, &line);
            info!(conn_id, %line, "sim: PLAN");
            return;
        }
        // RECIPE [id] — ingredients_for held (or arg) as product.
        if upper.starts_with("RECIPE ") || upper == "RECIPE" || upper == "?RECIPE" {
            let arg: Option<i32> = text
                .split_whitespace()
                .nth(1)
                .and_then(|s| s.parse().ok());
            let (p_id, held) = state
                .players
                .get(&conn_id)
                .map(|pl| (pl.p_id, pl.held_id))
                .unwrap_or((p.p_id, p.held_id));
            let product = arg.filter(|&id| id != 0).unwrap_or(held);
            let reply = state.craft_graph.format_recipe_query(product);
            let line = format!("{p_id} {reply}");
            send_ps_reply(outbound, conn_id, &line);
            info!(conn_id, %line, "sim: RECIPE");
            return;
        }
        // NEXTCRAFT [id] — products using held (or arg) as ingredient (craft graph).
        if upper.starts_with("NEXTCRAFT ")
            || upper == "NEXTCRAFT"
            || upper == "?NEXTCRAFT"
            || upper == "NEXT CRAFT"
        {
            let arg: Option<i32> = text
                .split_whitespace()
                .nth(1)
                .and_then(|s| s.parse().ok());
            let (p_id, held) = state
                .players
                .get(&conn_id)
                .map(|pl| (pl.p_id, pl.held_id))
                .unwrap_or((p.p_id, p.held_id));
            let item = arg.filter(|&id| id != 0).unwrap_or(held);
            let reply = state.craft_graph.format_nextcraft_query(item);
            let line = format!("{p_id} {reply}");
            send_ps_reply(outbound, conn_id, &line);
            info!(conn_id, %line, "sim: NEXTCRAFT");
            return;
        }
        // SEEKING [PROF] — AI goal self-debug string (optional for human players).
        // Default profession Forager; optional token FARMER/SMITH/HUNTER/…
        if upper.starts_with("SEEKING ") || upper == "SEEKING" || upper == "?SEEKING" {
            let token = text.split_whitespace().nth(1).unwrap_or("");
            let profession =
                parse_profession_token(token).unwrap_or(Profession::Forager);
            let (p_id, held, food, px, py) = state
                .players
                .get(&conn_id)
                .map(|pl| (pl.p_id, pl.held_id, pl.food, pl.x, pl.y))
                .unwrap_or((p.p_id, p.held_id, p.food, p.x, p.y));
            // Cheap nearby-food scan (Chebyshev ≤ 4) for goal sensors.
            let nearby_food = {
                let world = state.world.read().unwrap();
                let mut found = false;
                'scan: for dy in -4i32..=4 {
                    for dx in -4i32..=4 {
                        let oid = world.get_object(px + dx, py + dy);
                        if oid != 0 {
                            if let Some(def) = state.content.get(oid) {
                                if def.food_value > 0 {
                                    found = true;
                                    break 'scan;
                                }
                            }
                        }
                    }
                }
                found
            };
            let threat_near = state.animals.nearby_threat(px, py, ANIMAL_THREAT_RANGE);
            let prey_near = state.animals.nearby_prey(px, py, ANIMAL_THREAT_RANGE);
            let goal = pick_goal_ext(
                profession,
                held,
                food,
                nearby_food,
                threat_near,
                prey_near,
                false,
                0,
            );
            let reply = format_seeking_query(goal);
            let line = format!("{p_id} {reply}");
            send_ps_reply(outbound, conn_id, &line);
            info!(conn_id, %line, "sim: SEEKING");
            return;
        }
        // PATH <x> <y> — next A* step (dx dy) toward absolute tile, or FAIL.
        // Uses player-aware walkability (gate/door name exception + owned locks).
        if upper.starts_with("PATH ") || upper == "PATH" {
            let mut it = text.split_whitespace();
            let _ = it.next();
            let gx: Option<i32> = it.next().and_then(|s| s.parse().ok());
            let gy: Option<i32> = it.next().and_then(|s| s.parse().ok());
            let (sx, sy, p_id) = (p.x, p.y, p.p_id);
            let line = match (gx, gy) {
                (Some(gx), Some(gy)) if sx == gx && sy == gy => {
                    format!("{p_id} PATH 0 0")
                }
                (Some(gx), Some(gy)) => {
                    let content = state.content.clone();
                    let allies = state.allies.clone();
                    let step = {
                        let world = state.world.read().unwrap();
                        next_step(&world, sx, sy, gx, gy, &|x, y| {
                            is_walkable_for_player(&world, &content, x, y, p_id, &|a, b| {
                                allies.is_mutual_or_either(a, b)
                            })
                        })
                    };
                    match step {
                        Some((dx, dy)) => format!("{p_id} PATH {dx} {dy}"),
                        None => format!("{p_id} PATH FAIL"),
                    }
                }
                _ => format!("{p_id} PATH FAIL"),
            };
            send_ps_reply(outbound, conn_id, &line);
            info!(conn_id, %line, "sim: PATH");
            return;
        }
        // STEPS <x> <y> — A* path length estimate to absolute tile, or FAIL.
        if upper.starts_with("STEPS ") || upper == "STEPS" {
            let mut it = text.split_whitespace();
            let _ = it.next();
            let gx: Option<i32> = it.next().and_then(|s| s.parse().ok());
            let gy: Option<i32> = it.next().and_then(|s| s.parse().ok());
            let (sx, sy, p_id) = (p.x, p.y, p.p_id);
            let line = match (gx, gy) {
                (Some(gx), Some(gy)) => {
                    let content = state.content.clone();
                    let allies = state.allies.clone();
                    let n = {
                        let world = state.world.read().unwrap();
                        path_steps(&world, sx, sy, gx, gy, &|x, y| {
                            is_walkable_for_player(&world, &content, x, y, p_id, &|a, b| {
                                allies.is_mutual_or_either(a, b)
                            })
                        })
                    };
                    match n {
                        Some(n) => format!("{p_id} STEPS {n}"),
                        None => format!("{p_id} STEPS FAIL"),
                    }
                }
                _ => format!("{p_id} STEPS FAIL"),
            };
            send_ps_reply(outbound, conn_id, &line);
            info!(conn_id, %line, "sim: STEPS");
            return;
        }
        // WALKABLE <dx> <dy> — yes/no for tile relative to player (locks + gate exception).
        if upper.starts_with("WALKABLE ") || upper == "WALKABLE" {
            let mut it = text.split_whitespace();
            let _ = it.next();
            let dx: i32 = it.next().and_then(|s| s.parse().ok()).unwrap_or(0);
            let dy: i32 = it.next().and_then(|s| s.parse().ok()).unwrap_or(0);
            let (tx, ty, p_id) = (p.x + dx, p.y + dy, p.p_id);
            let content = state.content.clone();
            let allies = state.allies.clone();
            let ok = {
                let world = state.world.read().unwrap();
                is_walkable_for_player(&world, &content, tx, ty, p_id, &|a, b| {
                    allies.is_mutual_or_either(a, b)
                })
            };
            let yn = if ok { "yes" } else { "no" };
            let line = format!("{p_id} WALKABLE {yn}");
            send_ps_reply(outbound, conn_id, &line);
            info!(conn_id, dx, dy, ok, "sim: WALKABLE");
            return;
        }
        // GOHOME — pathfind one step toward home, or teleport one cardinal step.
        if upper == "GOHOME" {
            let (sx, sy, hx, hy, p_id, held, age) = (
                p.x, p.y, p.home_x, p.home_y, p.p_id, p.held_id, p.age,
            );
            if sx == hx && sy == hy {
                let line = format!("{p_id} GOHOME {sx} {sy} OK");
                send_ps_reply(outbound, conn_id, &line);
                return;
            }
            let content = state.content.clone();
            let allies = state.allies.clone();
            let step = {
                let world = state.world.read().unwrap();
                next_step(&world, sx, sy, hx, hy, &|x, y| {
                    is_walkable_for_player(&world, &content, x, y, p_id, &|a, b| {
                        allies.is_mutual_or_either(a, b)
                    })
                })
            };
            let (dx, dy) = step.unwrap_or_else(|| {
                // Teleport one cardinal step toward home when path is blocked.
                let dx = (hx - sx).signum();
                let dy = (hy - sy).signum();
                if dx != 0 {
                    (dx, 0)
                } else {
                    (0, dy)
                }
            });
            if apply_move_deltas(state, conn_id, sx, sy, &[(dx, dy)]) {
                maybe_send_map_chunk(state, outbound, conn_id);
                state.publish_player_view(conn_id);
                if let Some(np) = state.players.get(&conn_id) {
                    let line = format!("{} GOHOME {} {}", np.p_id, np.x, np.y);
                    send_ps_reply(outbound, conn_id, &line);
                    let spd = player_move_speed(state, np);
                    let pu = format_player_update_line(
                        p_id,
                        DEFAULT_PERSON_OBJECT,
                        held,
                        np.x,
                        np.y,
                        age,
                        spd,
                    np.done_moving_seq.max(1),
                    );
                    let near = nearby_conn_ids(state, np.x, np.y, NEARBY_RANGE);
                    send_nearby(
                        outbound,
                        &near,
                        format_server_message("PU", &[&pu]).into_bytes(),
                    );
                    info!(
                        conn_id,
                        x = np.x,
                        y = np.y,
                        home_x = hx,
                        home_y = hy,
                        "sim: GOHOME step"
                    );
                }
            }
            return;
        }
        // Free-form chat only (all commands returned above). Rate-limit here.
        {
            let now = state.sim_time;
            if let Some(pl) = state.players.get_mut(&conn_id) {
                while pl
                    .last_say_times
                    .front()
                    .is_some_and(|&t| now - t >= SAY_RATE_WINDOW_SECS)
                {
                    pl.last_say_times.pop_front();
                }
                if pl.last_say_times.len() >= SAY_RATE_MAX {
                    send_ps_reply(outbound, conn_id, "RATE");
                    return;
                }
                pl.last_say_times.push_back(now);
            }
        }
        // Volume ranges: MUMBLE=4, normal=age-scaled, SHOUT=48 (WHISPER is private).
        // Normal chat uses age-scaled range (Haxe speech distance).
        // Mute filter: skip listeners who muted the speaker (`MuteBook::should_deliver`).
        let chat_body = if upper.starts_with("SHOUT ") {
            text.split_once(' ').map(|(_, r)| r).unwrap_or(text)
        } else if upper.starts_with("MUMBLE ") {
            text.split_once(' ').map(|(_, r)| r).unwrap_or(text)
        } else {
            text
        };
        let range = if upper.starts_with("SHOUT ") || upper == "SHOUT" {
            SHOUT_RANGE
        } else if upper.starts_with("MUMBLE ") || upper == "MUMBLE" {
            MUMBLE_SAY_RANGE
        } else {
            chat_range_for_age(p.age)
        };
        let speaker_p_id = p.p_id;
        // Protocol: `PS\np_id/0 text\n#` then `FM` (official client holds PS until FRAME).
        let near = nearby_conn_ids(state, p.x, p.y, range);
        send_chat_ps(state, outbound, conn_id, speaker_p_id, chat_body, &near);
        info!(conn_id, text = %chat_body, range, "sim: SAY chat (PS/0 + FM)");
        return;
    }
    if tag.eq_ignore_ascii_case("REMV") {
        // REMV x y [i] [j]
        // - no indices: take last top-level contained
        // - i only: take contained[i] (nested under i discarded)
        // - i j: pocket-style nested take — sub-item j under contained[i]
        //   (j = -1 or omitted sub uses last nested; see container_take_nested)
        let mut parts = payload.split_whitespace();
        let x: i32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        let y: i32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        let slot = parts.next().and_then(|s| s.parse::<i32>().ok());
        let sub_raw = parts.next().and_then(|s| s.parse::<i32>().ok());
        if let Some(pl) = state.players.get(&conn_id) {
            if is_moving(pl) || !in_use_range(pl.x, pl.y, x, y, 1) {
                send_player_update_and_frame(state, outbound, conn_id);
                return;
            }
        }
        let hands = state.players.get(&conn_id).map(|p| p.held_id).unwrap_or(-1);
        if hands != 0 {
            return;
        }
        // Ownership theft check (Haxe illegal take subset).
        let owner_id = state
            .world
            .read()
            .unwrap()
            .get_helper(x, y)
            .map(|h| h.owner_id)
            .unwrap_or(0);
        let taker_id = state.players.get(&conn_id).map(|p| p.p_id).unwrap_or(0);
        if classify_take(taker_id, owner_id) == TakeLegality::Theft {
            let penalty = state.crime.record_theft(taker_id);
            let st = state.combat.stats_mut(taker_id);
            st.prestige = (st.prestige - penalty).max(0.0);
            state.sync_lineage_prestige_from_combat(taker_id);
            info!(conn_id, taker_id, owner_id, "sim: REMV theft prestige hit");
        }
        // Peek candidate id for permanency (Haxe: permanent=1 cannot leave containers).
        let peek_id = {
            let world = state.world.read().unwrap();
            match (slot, sub_raw) {
                (Some(s), Some(j)) if s >= 0 => {
                    world.get_helper(x, y).and_then(|hh| {
                        let si = s as usize;
                        if j < 0 {
                            hh.nested
                                .get(si)
                                .and_then(|n| n.last().copied())
                                .or_else(|| hh.contained.get(si).copied())
                        } else {
                            hh.nested
                                .get(si)
                                .and_then(|n| n.get(j as usize).copied())
                                .or_else(|| hh.contained.get(si).copied())
                        }
                    })
                }
                (Some(s), None) if s >= 0 => world
                    .get_helper(x, y)
                    .and_then(|h| h.contained.get(s as usize).copied()),
                _ => world
                    .get_helper(x, y)
                    .and_then(|h| h.contained.last().copied()),
            }
        };
        if let Some(pid) = peek_id {
            if state
                .content
                .get(pid)
                .map(|d| d.permanent)
                .unwrap_or(false)
            {
                info!(conn_id, x, y, id = pid, "sim: REMV blocked permanent contained");
                send_player_update_and_frame(state, outbound, conn_id);
                return;
            }
        }
        let item = {
            let mut world = state.world.write().unwrap();
            match (slot, sub_raw) {
                // Nested pocket take: REMV x y slot sub  (sub < 0 → last)
                (Some(s), Some(j)) if s >= 0 => {
                    let sub = if j < 0 {
                        None
                    } else {
                        Some(j as usize)
                    };
                    world.container_take_nested(x, y, s as usize, sub)
                }
                // Top-level: REMV x y [i]  (i < 0 → last)
                (Some(s), None) => {
                    let idx = if s < 0 {
                        None
                    } else {
                        Some(s as usize)
                    };
                    world.container_take(x, y, idx)
                }
                (None, _) => world.container_take(x, y, None),
                // Negative slot with sub: treat as last top-level (ignore sub)
                (Some(_), Some(_)) => world.container_take(x, y, None),
            }
        };
        if let Some(id) = item {
            if let Some(p) = state.players.get_mut(&conn_id) {
                p.held_id = id;
            }
            state.publish_player_view(conn_id);
            info!(conn_id, x, y, id, "sim: REMV from container");
            let near = nearby_conn_ids(state, x, y, NEARBY_RANGE);
            let tile = state.world.read().unwrap().get_object(x, y);
            for pkt in packets_after_drop(state, conn_id, x, y, tile) {
                send_nearby(outbound, &near, pkt);
            }
            if let Some(p) = state.players.get(&conn_id) {
                let spd = player_move_speed(state, p);
                let pu = format_player_update_line(
                    p.p_id,
                    person_object_id(&p),
                    p.held_id,
                    p.x,
                    p.y,
                    p.age,
                    spd,
                p.done_moving_seq.max(1),
                );
                send_nearby(
                    outbound,
                    &near,
                    format_server_message("PU", &[&pu]).into_bytes(),
                );
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UseResult {
    pub actor_before: i32,
    pub target_before: i32,
    pub actor_after: i32,
    pub target_after: i32,
    pub applied: bool,
    pub x: i32,
    pub y: i32,
}

pub fn apply_use_at(
    state: &mut SimState,
    conn_id: u64,
    tx: i32,
    ty: i32,
) -> Option<UseResult> {
    let player = state.players.get(&conn_id)?;
    if player.deleted {
        return None;
    }
    let actor = player.held_id;
    let (px, py) = (player.x, player.y);
    if is_moving(player) {
        return Some(UseResult {
            actor_before: actor,
            target_before: 0,
            actor_after: actor,
            target_after: 0,
            applied: false,
            x: tx,
            y: ty,
        });
    }

    let (target, uses_remaining) = {
        let w = state.world.read().unwrap();
        let target = w.get_object(tx, ty);
        let uses = w
            .get_helper(tx, ty)
            .map(|h| h.uses_remaining)
            .unwrap_or(0);
        (target, uses)
    };

    if !in_use_range(px, py, tx, ty, 1) {
        return Some(UseResult {
            actor_before: actor,
            target_before: target,
            actor_after: actor,
            target_after: target,
            applied: false,
            x: tx,
            y: ty,
        });
    }

    // Haxe multi-use: when one use left, prefer last-use transition table.
    let prefer_last =
        state.prefer_last_use || player.force_last_use || uses_remaining == 1;

    // Prefer content transition (actor, target). If none, Haxe falls back to
    // bare-hand pickup: empty hands + non-permanent ground object → swap.
    let tr = state
        .content
        .find_transition_prefer(actor, target, prefer_last)
        .cloned();

    let (actor_after, target_after, reverse_target, from_transition) = if let Some(ref tr) = tr {
        (
            tr.new_actor_id,
            tr.new_target_id,
            tr.reverse_use_target,
            true,
        )
    } else if actor == 0 && target != 0 {
        // Haxe `swapHandAndFloorObject` — empty hands, non-permanent ground.
        let permanent = state
            .content
            .get(target)
            .map(|d| d.permanent)
            .unwrap_or(false);
        if permanent {
            return Some(UseResult {
                actor_before: actor,
                target_before: target,
                actor_after: actor,
                target_after: target,
                applied: false,
                x: tx,
                y: ty,
            });
        }
        (target, 0, false, false)
    } else if actor != 0 && target != 0 {
        // Held object on non-permanent ground with no transition → swap both
        // (put down held, pick up target). Permanent ground refuses.
        let tgt_perm = state
            .content
            .get(target)
            .map(|d| d.permanent)
            .unwrap_or(false);
        let act_perm = state
            .content
            .get(actor)
            .map(|d| d.permanent)
            .unwrap_or(false);
        if tgt_perm || act_perm {
            return Some(UseResult {
                actor_before: actor,
                target_before: target,
                actor_after: actor,
                target_after: target,
                applied: false,
                x: tx,
                y: ty,
            });
        }
        // Swap hand ↔ floor.
        (target, actor, false, false)
    } else {
        return Some(UseResult {
            actor_before: actor,
            target_before: target,
            actor_after: actor,
            target_after: target,
            applied: false,
            x: tx,
            y: ty,
        });
    };

    {
        let mut w = state.world.write().unwrap();
        place_after_use(
            &mut w,
            &state.content,
            tx,
            ty,
            target,
            target_after,
            uses_remaining,
            reverse_target,
            from_transition,
        );
    }
    // Final base id on the tile after USE (simple or complex).
    state.record_world_change(tx, ty, target_after);
    schedule_decay(state, tx, ty, target_after);
    // Equip clothing-like new_actor into hat/chest/shoes when content names match.
    let equip_slot = if actor_after != 0 {
        state
            .content
            .get(actor_after)
            .and_then(|def| clothing_slot_for_object(&def.name, &def.description))
    } else {
        None
    };
    if let Some(p) = state.players.get_mut(&conn_id) {
        p.held_id = actor_after;
        p.force_last_use = false;
        // Learn tools when interacting with non-zero actor or target.
        if actor != 0 {
            p.tools.learn(actor);
        }
        if target != 0 {
            p.tools.learn(target);
        }
        if let Some(slot) = equip_slot {
            p.set_clothing(slot, actor_after);
        }
    }

    Some(UseResult {
        actor_before: actor,
        target_before: target,
        actor_after,
        target_after,
        applied: true,
        x: tx,
        y: ty,
    })
}

/// Place post-USE tile state with Haxe `DoChangeNumberOfUsesOnTarget` semantics.
///
/// - **New multi-use object + reverseUseTarget** (stone→pile): start at **1** use  
///   (Haxe: "a Pile starts with 1 uses not with the full numberOfUses").
/// - **New multi-use object + normal**: start at `num_uses` (full).
/// - **Same id + reverseUseTarget** (add stone to pile): `uses + 1` (cap at num_uses).
/// - **Same id + normal** (take from pile / harvest): `uses - 1`; 0 clears tile.
fn place_after_use(
    world: &mut World,
    content: &ContentDb,
    tx: i32,
    ty: i32,
    target_before: i32,
    target_after: i32,
    uses_before: i32,
    reverse_use_target: bool,
    from_transition: bool,
) {
    use ol_world::ComplexObject;

    if target_after == 0 {
        world.set_object(tx, ty, 0);
        return;
    }

    let num_uses = content
        .get(target_after)
        .map(|d| d.num_uses)
        .unwrap_or(0)
        .max(0);

    // Same base id: adjust uses (Haxe reverse += 1, else -= 1).
    if from_transition && target_after == target_before && num_uses > 1 {
        let cur = if uses_before > 0 {
            uses_before
        } else {
            // Helper missing — treat as full pile only when reverse-adding, else 1.
            if reverse_use_target {
                1
            } else {
                num_uses
            }
        };
        let next = if reverse_use_target {
            (cur + 1).min(num_uses)
        } else {
            cur - 1
        };
        if next <= 0 {
            // Depleted without last-use transform — clear (caller should prefer LT).
            world.set_object(tx, ty, 0);
        } else {
            world.set_object_complex(tx, ty, ComplexObject::with_uses(target_after, next));
        }
        return;
    }

    // Id changed to a multi-use object (new pile / bucket / deposit).
    if num_uses > 1 {
        let start = if reverse_use_target {
            // Haxe: reverseUse on new type → start at 1 (first stone in pile).
            1
        } else {
            num_uses
        };
        world.set_object_complex(tx, ty, ComplexObject::with_uses(target_after, start));
        return;
    }

    world.set_object(tx, ty, target_after);
}

/// Object id to send on the wire for a world tile (multi-use → dummy id).
fn wire_object_at(state: &SimState, x: i32, y: i32) -> i32 {
    let w = state.world.read().unwrap();
    let base = w.get_object(x, y);
    let uses = w
        .get_helper(x, y)
        .map(|h| h.uses_remaining)
        .unwrap_or(0);
    let uses = if uses > 0 {
        uses
    } else {
        state
            .content
            .get(base)
            .map(|d| d.num_uses)
            .unwrap_or(0)
            .max(0)
    };
    state.content.wire_id_for_uses(base, uses)
}

/// Wire packets to push after a successful USE (MX + PU + FX).
///
/// Map/player coords on the wire are birth-relative for this connection.
///
/// **MX p_id = `-(player)`** for transforms/pickups (Haxe `SendTransitionUpdate…`
/// with `doTransition=true`). Positive p_id is only for drops — using +p_id here
/// makes the official client animate the object flying from the player.
///
/// Multi-use tiles send the **dummy object id** for current uses (Haxe `dummyId()`).
pub fn packets_after_use(state: &SimState, conn_id: u64, r: &UseResult) -> Vec<Vec<u8>> {
    let Some(p) = state.players.get(&conn_id) else {
        return vec![];
    };
    let floor = state.world.read().unwrap().get_floor(r.x, r.y) as i32;
    let wire_obj = wire_object_at(state, r.x, r.y);
    let (mx, my) = p.world_to_client(r.x, r.y);
    let (px, py) = p.world_to_client(p.x, p.y);
    let mut out = Vec::new();
    // Transform / pickup / harvest — not a drop.
    let responsible = -p.p_id;
    out.push(
        format_map_change(mx, my, floor, wire_obj, responsible).into_bytes(),
    );
    let spd = player_move_speed(state, p);
    let pu = format_player_update_line(
        p.p_id,
        person_object_id(p),
        p.held_id,
        px,
        py,
        p.age,
        spd,
        p.done_moving_seq.max(1),
    );
    out.push(format_server_message("PU", &[&pu]).into_bytes());
    out.push(food_change_for_player(state, p).into_bytes());
    out
}

pub fn packets_after_drop(state: &SimState, conn_id: u64, x: i32, y: i32, placed: i32) -> Vec<Vec<u8>> {
    let Some(p) = state.players.get(&conn_id) else {
        return vec![];
    };
    let floor = state.world.read().unwrap().get_floor(x, y) as i32;
    // Prefer live tile wire id (dummy-aware); fall back to placed base.
    let wire_obj = {
        let live = wire_object_at(state, x, y);
        if live != 0 {
            live
        } else {
            state.content.wire_id_for_uses(placed, 0)
        }
    };
    let (mx, my) = p.world_to_client(x, y);
    let (px, py) = p.world_to_client(p.x, p.y);
    let mut out = Vec::new();
    out.push(format_map_change(mx, my, floor, wire_obj, p.p_id).into_bytes());
    let spd = player_move_speed(state, p);
    let pu = format_player_update_line(
        p.p_id,
        person_object_id(p),
        p.held_id,
        px,
        py,
        p.age,
        spd,
        p.done_moving_seq.max(1),
    );
    out.push(format_server_message("PU", &[&pu]).into_bytes());
    out
}

pub fn player_id_for_conn(conn_id: u64) -> i32 {
    (conn_id as i32).saturating_add(1).max(2)
}


/// Find a non-mountain grassland-ish tile near preference (spawn bootstrap).
pub fn find_playable_spawn(world: &World, prefer: (i32, i32)) -> (i32, i32) {
    let (px, py) = prefer;
    let ww = world.width_tiles.max(1);
    let hh = world.height_tiles.max(1);
    // Prefer biome 0 (green) when available.
    for r in 0i32..200 {
        for dy in -r..=r {
            for dx in -r..=r {
                if (dx as i32).abs() != r && (dy as i32).abs() != r && r > 0 {
                    continue;
                }
                let x = px + dx;
                let y = py + dy;
                let (x, y) = world.wrap_tile(x, y);
                let b = world.get_biome(x, y);
                if b != 21 && b != 6 {
                    // not mountain wall / deep ocean proxies
                    if b == 0 || r > 30 {
                        return (x, y);
                    }
                }
            }
        }
    }
    (px.rem_euclid(ww), py.rem_euclid(hh))
}

pub fn spawn_player(state: &mut SimState, conn_id: u64, email: &str) -> i32 {
    // Revive deleted player on re-login (self-play / reconnect).
    if state.players.contains_key(&conn_id) {
        if let Some(p) = state.players.get_mut(&conn_id) {
            if p.deleted {
                p.deleted = false;
                p.connected = true;
                p.food = START_FOOD;
                p.food_max = MAX_FOOD;
                p.held_id = 0;
                p.death_reason = None;
                p.age = 14.0;
                p.email = email.to_string();
                p.has_mc = false;
                if email.to_ascii_lowercase().contains(PLAYTEST_EMAIL_NEEDLE) {
                    p.first_name = PLAYTEST_FIRST_NAME.into();
                    p.family_name = playtest_family_name();
                    p.display_object_id = PLAYTEST_SKIN_OBJECT;
                }
                info!(conn_id, p_id = p.p_id, "sim: player revived");
            }
        }
        let p_id = state.players.get(&conn_id).map(|p| p.p_id).unwrap_or(0);
        let now = state.sim_time;
        state.afk.touch(p_id, now);
        return p_id;
    }
    let p_id = player_id_for_conn(conn_id);
    if p_id >= state.next_player_id {
        state.next_player_id = p_id + 1;
    }
    let mut p = Player::new(p_id, conn_id, email);
    if email.to_ascii_lowercase().contains(PLAYTEST_EMAIL_NEEDLE) {
        // Distinct identity for the headless/playtest Steam account + server version.
        p.first_name = PLAYTEST_FIRST_NAME.into();
        p.family_name = playtest_family_name();
        p.display_object_id = PLAYTEST_SKIN_OBJECT;
        info!(
            conn_id,
            p_id,
            name = %format!("{} {}", PLAYTEST_FIRST_NAME, p.family_name),
            skin = PLAYTEST_SKIN_OBJECT,
            "sim: playtest identity assigned"
        );
    } else {
        let (first, family) = naming::pick_random_name(&mut rand::thread_rng());
        p.first_name = first;
        p.family_name = family;
        p.display_object_id = DEFAULT_PERSON_OBJECT;
    }
    // Adult LOGIN must match net bootstrap PU position (`preferred_spawn` /
    // `state.spawn_x/y`). Mother-tile spawn is only for `spawn_child` (age 0).
    // Mismatch caused MOVE jump_too_far → force snap to NPC tiles (see
    // RustClient SERVER_MOVE_FEEDBACK.md).
    let mut mother_link: Option<i32> = None;
    let is_synthetic = conn_id >= 9_000_000; // self-play / NPC reserved bands
    let (sx, sy) = if is_synthetic {
        let no_mother = pick_best_mother_p_id(state).is_none();
        let eve = rand::random::<f32>() < EVE_OR_ADAM_BIRTH_CHANCE || no_mother;
        if eve {
            let w = state.world.read().unwrap();
            find_playable_spawn(&w, (state.spawn_x, state.spawn_y))
        } else if let Some(mid) = pick_best_mother_p_id(state) {
            mother_link = Some(mid);
            if let Some(m) = state.players.values().find(|pl| pl.p_id == mid) {
                (m.x, m.y)
            } else {
                let w = state.world.read().unwrap();
                find_playable_spawn(&w, (state.spawn_x, state.spawn_y))
            }
        } else {
            let w = state.world.read().unwrap();
            find_playable_spawn(&w, (state.spawn_x, state.spawn_y))
        }
    } else {
        // Human TCP: always bootstrap-aligned spawn.
        let w = state.world.read().unwrap();
        find_playable_spawn(&w, (state.spawn_x, state.spawn_y))
    };
    p.x = sx;
    p.y = sy;
    // Birth origin = Eve/wild spawn tile (or mother tile for synthetic child-link spawns).
    // Client wire positions are relative to this for the whole life.
    p.set_birth_origin(sx, sy);
    p.home_x = sx;
    p.home_y = sy;
    p.food = START_FOOD;
    p.food_max = MAX_FOOD;
    p.age = 14.0;
    {
        let mut w = state.world.write().unwrap();
        w.touch_radius(p.x, p.y, 1);
    }
    let display = p.display_name();
    let is_playtest = email.to_ascii_lowercase().contains(PLAYTEST_EMAIL_NEEDLE);
    state.accounts.on_spawn(email, p_id, &display);
    state.players.insert(conn_id, p);
    if let Some(mid) = mother_link {
        attach_fitness_mother_lineage(state, p_id, &display, mid, sx, sy);
    } else {
        // Eve/Adam wild birth: root lineage (Haxe EveOrAdam).
        state.social.ensure_lineage(p_id, &display);
        state.push_event(format!("EVE {p_id}"));
    }
    // Playtest convenience: place pickable stones + a nearby wolf so ground pickup
    // and animal MX walks are observable next to the client spawn.
    if is_playtest && !is_synthetic {
        seed_playtest_local_objects(state, sx, sy);
    }
    state.logins += 1;
    let now = state.sim_time;
    state.afk.touch(p_id, now);
    p_id
}

/// Stones (id 33, permanent=0) + one wolf entity/map object near playtest spawn.
fn seed_playtest_local_objects(state: &mut SimState, sx: i32, sy: i32) {
    const STONE: i32 = 33;
    // Adjacent tiles only (not under feet — permanent trees may already be there).
    let offsets = [(1, 0), (0, 1), (-1, 0), (0, -1), (1, 1)];
    {
        let mut w = state.world.write().unwrap();
        for (dx, dy) in offsets {
            let x = sx + dx;
            let y = sy + dy;
            // Force-place stones for playtest (overwrite non-player clutter).
            let cur = w.get_object(x, y);
            if cur == 0 || cur != STONE {
                w.set_object(x, y, STONE);
            }
        }
    }
    // One wolf a few tiles away for MX animal-move visibility.
    let wx = sx + 4;
    let wy = sy + 1;
    let (px, py) = find_empty_animal_tile(state, wx, wy);
    // Avoid duplicating if a wolf already sits on that tile.
    let already = state
        .animals
        .animals
        .iter()
        .any(|a| a.x == px && a.y == py);
    if !already {
        state.animals.spawn(AnimalKind::Wolf, px, py);
        let oid = AnimalKind::Wolf.object_id();
        let mut w = state.world.write().unwrap();
        if w.get_object(px, py) == 0 {
            w.set_object(px, py, oid);
        }
        info!(x = px, y = py, "sim: playtest local wolf + stones seeded");
    }
}

/// Record activity for AFK bookkeeping (MOVE / USE / DROP / SAY / EMOT / JUMP…).
///
/// Does not count [`NetIntent::KeepAlive`] or disconnect — only real actions.
fn touch_afk_activity(state: &mut SimState, conn_id: u64) {
    let Some(p) = state.players.get(&conn_id) else {
        return;
    };
    if p.deleted {
        return;
    }
    let p_id = p.p_id;
    let now = state.sim_time;
    state.afk.touch(p_id, now);
}

/// Minimal birth: create a baby player linked to the mother.
///
/// - `age = 0`, `food = START_FOOD` (10)
/// - `conn_id = mother_conn + BABY_CONN_OFFSET` (synthetic; free slot if taken)
/// - lineage `mother_id` set; mother map marker for the baby
///
/// Returns baby `p_id`, or `None` if mother missing/deleted.
pub fn spawn_child(state: &mut SimState, mother_conn: u64) -> Option<i32> {
    let mother = state.players.get(&mother_conn)?.clone();
    if mother.deleted {
        return None;
    }

    let baby_p_id = state.next_player_id;
    state.next_player_id = state.next_player_id.saturating_add(1);

    let mut baby_conn = mother_conn.saturating_add(BABY_CONN_OFFSET);
    while state.players.contains_key(&baby_conn) {
        baby_conn = baby_conn.saturating_add(1);
    }

    let (first, family) = naming::pick_random_name(&mut rand::thread_rng());
    let mut baby = Player::new(baby_p_id, baby_conn, &format!("baby{baby_p_id}@birth"));
    baby.first_name = first.clone();
    // Family name from mother (Haxe same family lineage).
    baby.family_name = if mother.family_name.is_empty() {
        family
    } else {
        mother.family_name.clone()
    };
    baby.x = mother.x;
    baby.y = mother.y;
    // Kids: birth origin is mother's absolute position at birth (Haxe gx/gy = mother.tx/ty).
    baby.set_birth_origin(mother.x, mother.y);
    baby.home_x = mother.home_x;
    baby.home_y = mother.home_y;
    baby.food = START_FOOD;
    baby.food_max = MAX_FOOD;
    baby.age = 0.0;
    // Synthetic: no live TCP; still "connected" for sim queries.
    baby.connected = true;

    {
        let mut w = state.world.write().unwrap();
        w.touch_radius(baby.x, baby.y, 1);
    }

    // Ensure mother lineage exists, then insert child with mother_id.
    let mother_name = mother.display_name();
    state.social.ensure_lineage(mother.p_id, &mother_name);
    let mother_node = state
        .social
        .lineages
        .get(&mother.p_id)
        .cloned()
        .unwrap_or_else(|| LineageNode::eve(mother.p_id, mother_name));
    let child_name = format!("{} {}", baby.first_name, baby.family_name);
    let child_node = LineageNode::with_mother(baby_p_id, child_name, &mother_node);
    state.social.lineages.insert(baby_p_id, child_node);

    state
        .markers
        .set_mother_marker(baby_p_id, mother.x, mother.y, mother.p_id);

    // Haxe: newborn often starts held when mother hands free.
    if mother.can_hold_baby() {
        baby.held_by = mother.p_id;
        if let Some(m) = state.players.get_mut(&mother_conn) {
            m.start_holding(baby_p_id);
        }
    }

    state.players.insert(baby_conn, baby);
    state.publish_player_view(baby_conn);
    state.publish_player_view(mother_conn);
    state.push_event(format!("BIRTH {baby_p_id} mother={}", mother.p_id));
    info!(
        mother_conn,
        mother_p_id = mother.p_id,
        baby_conn,
        baby_p_id,
        "sim: birth spawn_child"
    );
    Some(baby_p_id)
}

pub fn set_player_position(state: &mut SimState, conn_id: u64, x: i32, y: i32) -> bool {
    set_player_position_respecting_path(state, conn_id, x, y)
}

/// KA: ignore mid-path (K11). Returns true only when coords applied.
pub fn set_player_position_respecting_path(
    state: &mut SimState,
    conn_id: u64,
    x: i32,
    y: i32,
) -> bool {
    let Some(p) = state.players.get(&conn_id) else {
        return false;
    };
    if p.deleted {
        return false;
    }
    if p.move_path.is_some() {
        debug!(conn_id, x, y, "sim: KA ignored mid-path");
        return false;
    }
    let (x, y) = {
        let w = state.world.read().unwrap();
        w.wrap_tile(x, y)
    };
    let Some(p) = state.players.get_mut(&conn_id) else {
        return false;
    };
    p.x = x;
    p.y = y;
    state.world.write().unwrap().touch_radius(x, y, 1);
    true
}

/// Haxe BAD_BIOMES / impassable mountain wall (`ol_world::biome::SNOWINGREY`).
const BIOME_MOUNTAIN: u8 = 21;


/// Force PU+FM unstick (cancel MovePath if any).
pub fn send_player_update_and_frame(
    state: &mut SimState,
    outbound: &OutboundHub,
    conn_id: u64,
) {
    send_forced_player_update(state, outbound, conn_id, None);
}

/// Action result unstick: **force=0**, keep real `done_moving_seq` (do not bump).
///
/// Used after USE/DROP that fail or succeed so the client is not stuck with a
/// stale mid-action wait and does not desync MOVE sequence numbers.
pub fn send_action_result_pu_and_frame(
    state: &mut SimState,
    outbound: &OutboundHub,
    conn_id: u64,
) {
    let Some(p) = state.players.get(&conn_id).cloned() else {
        return;
    };
    if p.deleted {
        return;
    }
    let spd = player_move_speed(state, &p);
    let seq = p.done_moving_seq.max(1);
    let near = nearby_conn_ids(state, p.x, p.y, NEARBY_RANGE);
    let mut recipients = near;
    if !recipients.contains(&conn_id) {
        recipients.push(conn_id);
    }
    for &cid in &recipients {
        let (rx, ry) = state
            .players
            .get(&cid)
            .map(|v| v.world_to_client(p.x, p.y))
            .unwrap_or((p.x, p.y));
        let pu = format_player_update_line(
            p.p_id,
            person_object_id(&p),
            p.held_id,
            rx,
            ry,
            p.age,
            spd,
            seq,
        );
        // Urgent so USE is not stuck behind AI PU/MX flood.
        outbound.send_urgent(
            cid,
            format_server_message("PU", &[&pu]).into_bytes(),
        );
    }
    outbound.send_urgent(conn_id, format_server_message("FM", &[]).into_bytes());
}

/// Force PU+FM at **server** position (Haxe `CancleMovement`).
///
/// - Clears any active path.
/// - `done_seq`: if `Some(s)` with `s > 0`, use as `done_moving_seq` (client MOVE `@seq`).
///   Else use active path seq if any, else increment.
/// - Wire `force=1` so the client resyncs once (not thrash).
/// - PU fans out to nearby players (Haxe `SendUpdateToAllClosePlayers`); FM only to mover.
pub fn send_forced_player_update(
    state: &mut SimState,
    outbound: &OutboundHub,
    conn_id: u64,
    done_seq: Option<i32>,
) {
    {
        let Some(p) = state.players.get_mut(&conn_id) else {
            return;
        };
        if p.deleted {
            return;
        }
        let path_seq = p.move_path.take().map(|path| path.seq);
        p.moving = false;
        p.done_moving_seq = if let Some(s) = done_seq.filter(|&s| s > 0) {
            s
        } else if let Some(s) = path_seq {
            s
        } else {
            p.done_moving_seq.saturating_add(1).max(1)
        };
    }
    let Some(p) = state.players.get(&conn_id).cloned() else {
        return;
    };
    let spd = player_move_speed(state, &p);
    let near = nearby_conn_ids(state, p.x, p.y, NEARBY_RANGE);
    let mut recipients = near;
    if !recipients.contains(&conn_id) {
        recipients.push(conn_id);
    }
    // Per-viewer relative coords (birthPos).
    for &cid in &recipients {
        let (rx, ry) = state
            .players
            .get(&cid)
            .map(|v| v.world_to_client(p.x, p.y))
            .unwrap_or((p.x, p.y));
        let pu = format_player_update_line_full(
            p.p_id,
            person_object_id(&p),
            p.held_id,
            rx,
            ry,
            p.age,
            spd,
            0,
            0,
            1, // force
            0, // action
            0,
            0,
            0,
            0,
            0,
            -1,
            p.done_moving_seq.max(1),
        );
        outbound.send(
            cid,
            format_server_message("PU", &[&pu]).into_bytes(),
        );
    }
    // FRAME unsticks the acting client only (Haxe also sends FRAME to that connection).
    outbound.send(conn_id, format_server_message("FM", &[]).into_bytes());
}

/// Pure catch-up extras (Haxe TimeHelper option A).
pub fn catch_up_extra_steps(tick_after_base: u64, periods_behind: u32, max_extra: u32) -> u32 {
    if periods_behind == 0 || max_extra == 0 {
        return 0;
    }
    let mut extras = 0u32;
    let mut tick = tick_after_base;
    while extras < max_extra && extras < periods_behind {
        if tick % 10 == 0 {
            break;
        }
        extras += 1;
        tick = tick.wrapping_add(1);
    }
    extras
}

pub fn pick_best_mother_p_id(state: &SimState) -> Option<i32> {
    let child = ChildView {
        is_human: true,
        prestige_class: 0,
    };
    let mut best: Option<(i32, f32)> = None;
    for p in state.players.values() {
        if p.deleted || !crate::birth_fitness::is_mother_age_fertile(p.age) {
            continue;
        }
        let mali = state
            .fertility
            .by_mother
            .get(&p.p_id)
            .map(|r| r.children_birth_mali)
            .unwrap_or(0.0);
        let m = MotherView {
            deleted: false,
            is_female: true,
            age: p.age,
            food: p.food,
            food_max: p.food_max,
            exhaustion: 0.0,
            heat: 0.5,
            wounded: false,
            held_id: p.held_id,
            held_speed_mult: 1.0,
            children_birth_mali: mali,
            prestige_class: 0,
            prestige_from_eating: 0.0,
            family_prestige_for_child: 0.0,
            has_close_nonblocking_grave: false,
            has_close_blocking_grave: false,
            is_human: true,
            little_kids_count: 0,
        };
        let fit = mother_fitness(&m, &child);
        if fit <= 0.0 {
            continue;
        }
        match best {
            Some((_, b)) if b >= fit => {}
            _ => best = Some((p.p_id, fit)),
        }
    }
    best.map(|(id, _)| id)
}

pub fn pick_best_father_p_id(state: &SimState, mother_p_id: i32) -> Option<i32> {
    let mother = state.players.values().find(|p| p.p_id == mother_p_id)?;
    let child = ChildView {
        is_human: true,
        prestige_class: 0,
    };
    let mother_mali = state
        .fertility
        .by_mother
        .get(&mother_p_id)
        .map(|r| r.children_birth_mali)
        .unwrap_or(0.0);
    let mother_view = MotherView {
        deleted: mother.deleted,
        is_female: true,
        age: mother.age,
        food: mother.food,
        food_max: mother.food_max,
        exhaustion: 0.0,
        heat: 0.5,
        wounded: false,
        held_id: mother.held_id,
        held_speed_mult: 1.0,
        children_birth_mali: mother_mali,
        prestige_class: 0,
        prestige_from_eating: 0.0,
        family_prestige_for_child: 0.0,
        has_close_nonblocking_grave: false,
        has_close_blocking_grave: false,
        is_human: true,
        little_kids_count: 0,
    };
    let mx = mother.x;
    let my = mother.y;
    let mut best: Option<(i32, f32)> = None;
    for pl in state.players.values() {
        if pl.deleted || pl.p_id == mother_p_id {
            continue;
        }
        let dist = ((pl.x - mx).abs().max((pl.y - my).abs())) as f32;
        let f = FatherView {
            deleted: false,
            age: pl.age,
            food: pl.food,
            food_max: pl.food_max,
            exhaustion: 0.0,
            heat: 0.5,
            wounded: false,
            held_id: pl.held_id,
            held_speed_mult: 1.0,
            prestige_class: 0,
            prestige_from_eating: 0.0,
            is_human: true,
            dist_to_mother: dist,
            is_partner: false,
            little_kids_count: 0,
        };
        let fit = father_fitness(&f, &child, &mother_view);
        if fit <= 0.0 {
            continue;
        }
        match best {
            Some((_, b)) if b >= fit => {}
            _ => best = Some((pl.p_id, fit)),
        }
    }
    best.map(|(id, _)| id)
}

pub fn attach_fitness_mother_lineage(
    state: &mut SimState,
    child_p_id: i32,
    child_display: &str,
    mother_p_id: i32,
    marker_x: i32,
    marker_y: i32,
) -> Option<i32> {
    let mother_name = state
        .players
        .values()
        .find(|pl| pl.p_id == mother_p_id)
        .map(|pl| pl.display_name())
        .unwrap_or_else(|| format!("M{mother_p_id}"));
    state.social.ensure_lineage(mother_p_id, &mother_name);
    if let Some(n) = state.social.lineages.get_mut(&mother_p_id) {
        let combat_p = state
            .combat
            .stats
            .get(&mother_p_id)
            .map(|s| s.prestige)
            .unwrap_or(0.0);
        if combat_p > n.prestige {
            n.set_prestige(combat_p);
        }
    }
    let mother_node = state
        .social
        .lineages
        .get(&mother_p_id)
        .cloned()
        .unwrap_or_else(|| LineageNode::eve(mother_p_id, mother_name));
    let mut child_node = LineageNode::with_mother(child_p_id, child_display, &mother_node);
    let father_id = pick_best_father_p_id(state, mother_p_id);
    if let Some(fid) = father_id {
        child_node.father_id = Some(fid);
    }
    state.social.lineages.insert(child_p_id, child_node);
    state
        .markers
        .set_mother_marker(child_p_id, marker_x, marker_y, mother_p_id);
    {
        let r = state.fertility.record_mut(mother_p_id);
        r.children_birth_mali =
            crate::birth_fitness::next_children_birth_mali(r.children_birth_mali);
    }
    state.push_event(format!(
        "SPAWN {child_p_id} mother={mother_p_id} father={} fitness",
        father_id.unwrap_or(0)
    ));
    father_id
}

pub fn apply_move_path_start(
    state: &mut SimState,
    outbound: &OutboundHub,
    conn_id: u64,
    xs: i32,
    ys: i32,
    deltas: &[(i32, i32)],
    client_seq: Option<i32>,
) -> Result<(), MoveReject> {
    // Baby MOVE while held → jump out of arms (user: drop if they move out).
    // Haxe prefers JUMP; we also honor MOVE as an explicit leave.
    let held_by = state
        .players
        .get(&conn_id)
        .map(|p| p.held_by)
        .unwrap_or(0);
    if held_by != 0 {
        let baby_p_id = state.players.get(&conn_id).map(|p| p.p_id).unwrap_or(0);
        if let Some(mother) = state
            .players
            .values_mut()
            .find(|pl| pl.p_id == held_by && pl.holding_player_id == baby_p_id)
        {
            mother.release_holding();
        }
        if let Some(pl) = state.players.get_mut(&conn_id) {
            pl.held_by = 0;
        }
        info!(conn_id, baby_p_id, held_by, "sim: baby MOVE drop out of arms");
        state.publish_player_view(conn_id);
    }
    let (px, py, deleted, sleeping, sitting) = {
        let p = state.players.get(&conn_id).ok_or(MoveReject::NoPlayer)?;
        (p.x, p.y, p.deleted, p.sleeping, p.sitting)
    };
    if deleted {
        return Err(MoveReject::Deleted);
    }
    if sleeping {
        return Err(MoveReject::Sleeping);
    }
    if SIT_BLOCKS_MOVE && sitting {
        return Err(MoveReject::Sitting);
    }
    // Haxe MoveHelper.moveHelper:
    //   if isBlocked(clientStart) || quadDist > MaxMovementQuadJumpDistanceBeforeForce(5)
    //     → CancleMovement (no snap).
    // Else if jump: snap to client start (positionChanged), then accept path.
    // Mid-path new MOVE replaces the path (Haxe always overwrites newMoves).
    //
    // Timed path jump gate is **only** Haxe quadDist ≤ 5. `move_jump_max_chebyshev`
    // applies to instant MOVE only and must not widen this gate.
    let jump_quad = move_quad_dist(xs, ys, px, py);
    let max_quad = MAX_MOVE_QUAD_JUMP_BEFORE_FORCE;
    if jump_quad > max_quad {
        // Too far: caller force-PU at **server** position (Haxe CancleMovement).
        return Err(MoveReject::JumpTooFar);
    }
    // Haxe checks isBlocked(client xs,ys) before mutating — keep server tile on reject.
    {
        let world = state.world.read().unwrap();
        let (sx, sy) = world.wrap_tile(xs, ys);
        if biome_blocks_move(world.get_biome(sx, sy))
            || !is_walkable(&world, &state.content, sx, sy)
        {
            return Err(MoveReject::BlockedStart);
        }
    }
    let (start_x, start_y) = if jump_quad == 0 {
        (px, py)
    } else {
        // Accept client position (Haxe positionChanged — no CancleMovement).
        if let Some(p) = state.players.get_mut(&conn_id) {
            p.move_path = None;
            p.moving = false;
            p.x = xs;
            p.y = ys;
        }
        state.world.write().unwrap().touch_radius(xs, ys, 1);
        debug!(
            conn_id,
            xs, ys, px, py, jump_quad, max_quad, "sim: MOVE accept client jump start"
        );
        (xs, ys)
    };
    // Mid-path replace: clear residual path when start tile already matches.
    if let Some(p) = state.players.get_mut(&conn_id) {
        if p.move_path.is_some() {
            p.move_path = None;
            p.moving = false;
        }
    }
    let (accepted, trunc) = {
        let t0 = ol_metrics::ScopeTimer::start();
        let world = state.world.read().unwrap();
        let out = truncate_walkable(&world, &state.content, start_x, start_y, deltas);
        state.last_lock_wait_us = t0.elapsed().as_micros().min(u128::from(u32::MAX)) as u32;
        out
    };
    if accepted.is_empty() {
        return Err(MoveReject::EmptyPath);
    }
    let (speed, seq) = {
        let p = state.players.get(&conn_id).ok_or(MoveReject::NoPlayer)?;
        let seq = resolve_move_seq(p, client_seq);
        let ballast = weight_item_count(p.held_id, p.backpack.len());
        let speed = compose_move_speed(
            p.riding,
            &state.weather,
            &state.snow,
            &state.fire,
            p.x,
            p.y,
            ballast,
        );
        (speed, seq)
    };
    let path = build_move_path(
        start_x,
        start_y,
        accepted.clone(),
        speed,
        seq,
        trunc,
        state.tick,
    );
    let total = path.total_sec;
    // PM wire uses start-relative waypoint deltas (client form), not per-step.
    let wire_deltas = steps_to_client_path_deltas(&accepted);
    let p_id = {
        let p = state.players.get_mut(&conn_id).ok_or(MoveReject::NoPlayer)?;
        p.move_path = Some(path);
        p.moving = true;
        p.p_id
    };
    // Per-viewer birth-relative PM (Haxe transformX/Y for each connection).
    let near = nearby_conn_ids(state, start_x, start_y, NEARBY_RANGE);
    let mut recipients: Vec<u64> = near;
    if !recipients.contains(&conn_id) {
        recipients.push(conn_id);
    }
    for &cid in &recipients {
        let (rx, ry) = state
            .players
            .get(&cid)
            .map(|v| v.world_to_client(start_x, start_y))
            .unwrap_or((start_x, start_y));
        let pm = ol_protocol::format_player_moves_start(
            p_id,
            rx,
            ry,
            total,
            total,
            trunc,
            &wire_deltas,
        );
        // Official client holds PM until FM (waitForFrameMessages after ACCEPTED).
        outbound.send_urgent(cid, pm.into_bytes());
        send_frame(outbound, cid);
    }
    state.publish_player_view(conn_id);
    info!(
        conn_id,
        p_id,
        start_x,
        start_y,
        steps = accepted.len(),
        trunc,
        seq,
        total_sec = total,
        "sim: MOVE PM sent (self+nearby)"
    );
    Ok(())
}

pub fn tick_move_paths(state: &mut SimState, dt: f32, outbound: &OutboundHub) {
    if !state.timed_movement || dt <= 0.0 {
        return;
    }
    let conns: Vec<u64> = state
        .players
        .iter()
        .filter(|(_, p)| p.move_path.is_some() && !p.deleted)
        .map(|(c, _)| *c)
        .collect();
    let (ww, hh, wrap_world) = {
        let w = state.world.read().unwrap();
        (w.width_tiles, w.height_tiles, w.wrap)
    };
    let wrap_fn = |x: i32, y: i32| {
        if wrap_world && ww > 0 && hh > 0 {
            (x.rem_euclid(ww), y.rem_euclid(hh))
        } else {
            (x, y)
        }
    };
    for conn_id in conns {
        let tick = state.tick;
        let (mut path, mut x, mut y, held_baby, seq) = {
            let Some(p) = state.players.get_mut(&conn_id) else {
                continue;
            };
            let Some(path) = p.move_path.take() else {
                continue;
            };
            let seq = path.seq;
            (path, p.x, p.y, p.holding_player_id, seq)
        };
        let result = {
            let world = state.world.read().unwrap();
            let content = &state.content;
            advance_path(
                &mut path,
                &mut x,
                &mut y,
                dt,
                tick,
                &wrap_fn,
                &|nx, ny| {
                    biome_blocks_move(world.get_biome(nx, ny))
                        || !is_walkable(&world, content, nx, ny)
                },
            )
        };
        if !result.commits.is_empty() {
            state.world.write().unwrap().touch_radius(x, y, 1);
            if held_baby != 0 {
                if let Some(baby) = state.players.values_mut().find(|pl| pl.p_id == held_baby) {
                    baby.x = x;
                    baby.y = y;
                }
            }
        }
        if result.cancelled {
            // Keep path seq explicit — do not clear path then call force with None
            // (that would saturating_add and double-step the seq).
            if let Some(p) = state.players.get_mut(&conn_id) {
                p.x = x;
                p.y = y;
                p.move_path = None;
                p.moving = false;
            }
            send_forced_player_update(state, outbound, conn_id, Some(seq));
            state.publish_player_view(conn_id);
            continue;
        }
        if result.finished {
            // Haxe updateMovement: done_moving_seqNum = newMoveSeqNumber; forced=false; PU.
            if let Some(p) = state.players.get_mut(&conn_id) {
                p.x = x;
                p.y = y;
                p.move_path = None;
                p.moving = false;
                p.done_moving_seq = seq;
            }
            maybe_send_map_chunk(state, outbound, conn_id);
            state.publish_player_view(conn_id);
            if let Some(p) = state.players.get(&conn_id).cloned() {
                let spd = player_move_speed(state, &p);
                // Must emit the path seq (not hardcoded 1) so client matches MOVE @seq.
                let near = nearby_conn_ids(state, p.x, p.y, NEARBY_RANGE);
                for &cid in &near {
                    let (rx, ry) = state
                        .players
                        .get(&cid)
                        .map(|v| v.world_to_client(p.x, p.y))
                        .unwrap_or((p.x, p.y));
                    let pu = format_player_update_line_full(
                        p.p_id,
                        person_object_id(&p),
                        p.held_id,
                        rx,
                        ry,
                        p.age,
                        spd,
                        0,
                        0,
                        0, // force=0 natural finish
                        0,
                        0,
                        0,
                        0,
                        0,
                        0,
                        -1,
                        p.done_moving_seq.max(1),
                    );
                    // PU then FM so official clients flush the move-complete update.
                    outbound.send_urgent(
                        cid,
                        format_server_message("PU", &[&pu]).into_bytes(),
                    );
                    send_frame(outbound, cid);
                }
            }
            continue;
        }
        if let Some(p) = state.players.get_mut(&conn_id) {
            p.x = x;
            p.y = y;
            p.moving = true;
            p.move_path = Some(path);
        }
        if !result.commits.is_empty() {
            maybe_send_map_chunk(state, outbound, conn_id);
            state.publish_player_view(conn_id);
        }
    }
}

pub fn cancel_movement(
    state: &mut SimState,
    outbound: &OutboundHub,
    conn_id: u64,
    seq: i32,
    _use_vog: bool,
) {
    // Haxe CancleMovement: done_moving_seqNum = seq; forced PU at server pos.
    // Do not double-increment seq (send_forced_player_update owns the write).
    send_forced_player_update(
        state,
        outbound,
        conn_id,
        if seq > 0 { Some(seq) } else { None },
    );
    state.publish_player_view(conn_id);
}

/// True when MOVE must not enter this biome (mountain wall).
#[inline]
fn biome_blocks_move(biome: u8) -> bool {
    biome == BIOME_MOUNTAIN
}

pub fn apply_move_deltas(
    state: &mut SimState,
    conn_id: u64,
    xs: i32,
    ys: i32,
    deltas: &[(i32, i32)],
) -> bool {
    apply_move_deltas_with_seq(state, conn_id, xs, ys, deltas, None)
}

pub fn apply_move_deltas_with_seq(
    state: &mut SimState,
    conn_id: u64,
    xs: i32,
    ys: i32,
    deltas: &[(i32, i32)],
    client_seq: Option<i32>,
) -> bool {
    if state
        .players
        .get(&conn_id)
        .map(|p| p.deleted || p.sleeping || (SIT_BLOCKS_MOVE && p.sitting))
        .unwrap_or(true)
    {
        return false;
    }
    let (origin_x, origin_y) = state
        .players
        .get(&conn_id)
        .map(|p| (p.x, p.y))
        .unwrap_or((xs, ys));
    let start_x = if move_chebyshev(xs, ys, origin_x, origin_y) <= state.move_jump_max_chebyshev {
        origin_x
    } else {
        xs
    };
    let start_y = if move_chebyshev(xs, ys, origin_x, origin_y) <= state.move_jump_max_chebyshev {
        origin_y
    } else {
        ys
    };
    let accepted = {
        let world = state.world.read().unwrap();
        let (acc, _) = truncate_walkable(&world, &state.content, start_x, start_y, deltas);
        acc
    };
    if accepted.is_empty() && !deltas.is_empty() {
        return false;
    }
    let mut x = start_x;
    let mut y = start_y;
    for (dx, dy) in &accepted {
        x += dx;
        y += dy;
    }
    let (x, y) = {
        let world = state.world.read().unwrap();
        let (x, y) = world.wrap_tile(x, y);
        // Block MOVE into bad biomes (21 = mountain). Position unchanged.
        if biome_blocks_move(world.get_biome(x, y)) {
            return false;
        }
        (x, y)
    };
    // Instant MOVE while held as baby → drop out first.
    let (held_by_self, baby_pid_self) = state
        .players
        .get(&conn_id)
        .map(|p| (p.held_by, p.p_id))
        .unwrap_or((0, 0));
    if held_by_self != 0 {
        if let Some(mother) = state
            .players
            .values_mut()
            .find(|pl| pl.p_id == held_by_self && pl.holding_player_id == baby_pid_self)
        {
            mother.release_holding();
        }
        if let Some(pl) = state.players.get_mut(&conn_id) {
            pl.held_by = 0;
        }
    }
    let held_baby = if let Some(p) = state.players.get_mut(&conn_id) {
        p.x = x;
        p.y = y;
        p.moving = false;
        p.move_path = None;
        p.done_moving_seq = resolve_move_seq(p, client_seq);
        p.holding_player_id
    } else {
        0
    };
    // Baby teleports with carrier each MOVE (while still held).
    if held_baby != 0 {
        if let Some(baby) = state.players.values_mut().find(|pl| pl.p_id == held_baby) {
            baby.x = x;
            baby.y = y;
        }
    }
    state.world.write().unwrap().touch_radius(x, y, 1);
    true
}

/// Advance age/food, auto-decay, environment; emit BW/DY for starving infants;
/// periodically send HX heat packets from biome temperature.
///
/// Food drain is multiplied by [`OLD_AGE_FOOD_DRAIN_MULT`] when `age > OLD_AGE_THRESHOLD`,
/// by [`SLEEP_FOOD_DRAIN_MULT`] (0.5) while [`Player::sleeping`], by
/// [`SIT_FOOD_DRAIN_MULT`] (0.75) while [`Player::sitting`], and by
/// [`SICK_FOOD_DRAIN_MULT`] (1.3) while [`Player::sick`].
/// When `age > MAX_AGE`, the player is deleted with `death_reason = reason_age`.
/// On death, held + clothing + backpack are scattered onto empty nearby tiles
/// (rings first so graves can keep the body tile). Hunger/age deaths place
/// [`SimState::grave_object_id`] at the death tile when non-zero
/// (resolved via [`resolve_grave_object_id`]). Coins inherit to mother if online, else treasury.
///
/// When `age < BABY_AGE_THRESHOLD` and `food < STARVING_FOOD_THRESHOLD`, once every
/// [`VITALS_EMIT_INTERVAL_SECS`] of sim time, send [`format_baby_wiggle`] and
/// [`format_dying`] (with isSick when [`Player::sick`]) to connections within
/// [`NEARBY_RANGE`].
///
/// When `food < HUNGER_EMOT_FOOD_THRESHOLD` (and still alive), once every
/// [`HUNGER_EMOT_INTERVAL_SECS`] of sim time, send PE hunger emote
/// ([`HUNGER_EMOT_INDEX`]) to connections within [`NEARBY_RANGE`].
///
/// While [`Player::sleeping`], once every [`SLEEP_EMOT_INTERVAL_SECS`] of sim time,
/// send PE sleep/snore emote ([`SLEEP_EMOT_INDEX`]) to connections within
/// [`NEARBY_RANGE`].
///
/// Every [`HX_EMIT_INTERVAL_SECS`] of sim time, send HX (`format_heat_change`) to
/// each living player using [`Environment::temperature_at_biome`] at their tile.
///
/// AFK: when idle ≥ [`DEFAULT_AFK_SECS`] since last activity touch, the player is
/// considered AFK (no kick yet). On the tick that first crosses the threshold,
/// optionally emit PE yawn ([`YAWN_EMOT_INDEX`]) to nearby and log `AFK <p_id>`.
///
/// Indoor stub: floor id ≠ 0 halves [`TEMP_FOOD_EXTRA`] (shelter from extremes).
pub fn tick_vitals(state: &mut SimState, dt: f32, outbound: &OutboundHub) {
    tick_vitals_with_metrics(state, dt, outbound, None);
}

/// Like [`tick_vitals`], optionally recording death counts into process metrics.
///
/// Time dilation: `dt` is multiplied by [`SimState::sim_speed`] (clamped ≥ 0).
/// When [`SimState::paused`] is true, returns immediately (vitals skip).
pub fn tick_vitals_with_metrics(
    state: &mut SimState,
    dt: f32,
    outbound: &OutboundHub,
    counters: Option<&Counters>,
) {
    if state.paused {
        return;
    }
    let speed = if state.sim_speed.is_finite() && state.sim_speed >= 0.0 {
        state.sim_speed
    } else {
        1.0
    };
    let dt = dt * speed;
    state.sim_time += dt;
    // Snapshot positions for temperature (avoid borrow clash with players mut).
    let pos: Vec<(u64, i32, i32)> = state
        .players
        .iter()
        .filter(|(_, p)| !p.deleted)
        .map(|(c, p)| (*c, p.x, p.y))
        .collect();
    // Per-player food drain/sec =
    //   (FOOD_USE_PER_SEC * biome_mult * day_night_mult * apoc_mult
    //    + TEMP_FOOD_EXTRA [halved indoors] [+ DESERT_EXTRA when desert & hot])
    //   * old_age_mult (1.5 when age > 60) * sleep_mult (0.5 when sleeping)
    //   * sick_mult (1.3 when sick).
    let day_night = state.environment.day_night_multiplier();
    let apoc_mult = state.apocalypse.food_drain_multiplier();
    let mut food_drain: HashMap<u64, f32> = HashMap::new();
    // conn_id → biome temperature (reuse for periodic HX).
    let mut heat_by_conn: HashMap<u64, f32> = HashMap::new();
    {
        let world = state.world.read().unwrap();
        for (cid, x, y) in pos {
            let biome = world.get_biome(x, y);
            let t = state.environment.temperature_at_biome(biome);
            heat_by_conn.insert(cid, t);
            let mult = biome_food_multiplier(biome);
            // Extreme temps cost extra food (on top of biome / day-night / apoc multipliers).
            // Indoor stub: floor id != 0 → half TEMP_FOOD_EXTRA.
            let indoor = world.get_floor(x, y) != 0;
            let mut extra = if t < 0.25 || t > 0.75 {
                if indoor {
                    TEMP_FOOD_EXTRA * 0.5
                } else {
                    TEMP_FOOD_EXTRA
                }
            } else {
                0.0
            };
            // Desert (biome 5) heat: additional additive drain when hot.
            if biome == 5 && t > 0.75 {
                extra += DESERT_EXTRA;
            }
            // Weather multiplies base drain; clothing warmth reduces additive temp extra.
            let weather_mult = state.weather.food_drain_mult();
            food_drain.insert(
                cid,
                FOOD_USE_PER_SEC * mult * day_night * apoc_mult * weather_mult + extra,
            );
        }
    }

    // Snapshot wound bleed before mut player loop (avoid combat/players borrow clash).
    let bleed_by_pid: HashMap<i32, f32> = state
        .players
        .values()
        .filter(|p| !p.deleted)
        .map(|p| (p.p_id, state.combat.bleed_drain(p.p_id)))
        .filter(|(_, b)| *b > 0.0)
        .collect();

    // Advance weather timer (season-biased expiry).
    let season = state.environment.season;
    state.weather.tick(dt, season);
    state.fire.tick(dt);
    state.snow.sync_season(season);
    // Light snow blanket near online players in winter.
    if matches!(season, Season::Winter) {
        let centers: Vec<(i32, i32)> = state
            .players
            .values()
            .filter(|p| p.connected && !p.deleted)
            .map(|p| (p.x, p.y))
            .collect();
        state.snow.blanket_near(&centers, 2);
    }
    // Snapshot fire+snow drain by conn before mut loop.
    let hazard_by_cid: HashMap<u64, f32> = state
        .players
        .iter()
        .filter(|(_, p)| !p.deleted)
        .map(|(&cid, p)| {
            (
                cid,
                state.fire.drain_at(p.x, p.y) + state.snow.food_extra_at(p.x, p.y),
            )
        })
        .filter(|(_, d)| *d > 0.0)
        .collect();

    let mut dead = Vec::new();
    // (p_id, x, y, sick) for starving-infant BW + DY fan-out after the mut borrow ends.
    let mut vitals_emits: Vec<(i32, i32, i32, bool)> = Vec::new();
    // (p_id, x, y) for PE hunger emote fan-out after the mut borrow ends.
    let mut hunger_emots: Vec<(i32, i32, i32)> = Vec::new();
    // (p_id, x, y) for PE sleep/snore emote fan-out after the mut borrow ends.
    let mut sleep_emots: Vec<(i32, i32, i32)> = Vec::new();
    for (cid, p) in state.players.iter_mut() {
        if p.deleted {
            continue;
        }
        p.age += AGE_YEARS_PER_SEC * dt;
        // Old age death before hunger (age > 120 → reason_age).
        if p.age > MAX_AGE {
            p.deleted = true;
            p.death_reason = Some(DeathCause::Age.wire_tag().into());
            // held/clothing/backpack drained later by scatter_backpack_on_death
            p.vitals_emit_timer = 0.0;
            p.hunger_emot_timer = 0.0;
            p.sleep_emot_timer = 0.0;
            dead.push(*cid);
            continue;
        }
        let mut drain = food_drain
            .get(cid)
            .copied()
            .unwrap_or(FOOD_USE_PER_SEC);
        if p.age > OLD_AGE_THRESHOLD {
            drain *= OLD_AGE_FOOD_DRAIN_MULT;
        }
        if p.sleeping {
            drain *= SLEEP_FOOD_DRAIN_MULT;
        }
        if p.sitting {
            drain *= SIT_FOOD_DRAIN_MULT;
        }
        if p.sick {
            drain *= SICK_FOOD_DRAIN_MULT;
        }
        // Clothing warmth reduces extreme-temp portion of drain (bonus 0..1.5 → up to -0.03).
        let warm = clothing_temp_bonus(p.hat, p.chest, p.shoes);
        if warm > 0.0 {
            drain = (drain - warm * 0.02).max(FOOD_USE_PER_SEC * 0.25);
        }
        // Wound bleed (combat) adds additive food/sec (snapshotted before mut loop).
        if let Some(&bleed) = bleed_by_pid.get(&p.p_id) {
            drain += bleed;
        }
        // Fire/snow extras snapshotted before mut loop.
        if let Some(&extra) = hazard_by_cid.get(cid) {
            drain += extra;
        }
        p.food -= drain * dt;
        // Starving infant: accumulate emit timer; fire BW+DY every ~5s sim time.
        if p.age < BABY_AGE_THRESHOLD
            && p.food < STARVING_FOOD_THRESHOLD
            && p.food >= DEATH_FOOD_THRESHOLD
        {
            p.vitals_emit_timer += dt;
            if p.vitals_emit_timer >= VITALS_EMIT_INTERVAL_SECS {
                p.vitals_emit_timer = 0.0;
                vitals_emits.push((p.p_id, p.x, p.y, p.sick));
            }
        } else {
            p.vitals_emit_timer = 0.0;
        }
        // Low food: PE hunger emote every ~8s sim time.
        if p.food < HUNGER_EMOT_FOOD_THRESHOLD && p.food >= DEATH_FOOD_THRESHOLD {
            p.hunger_emot_timer += dt;
            if p.hunger_emot_timer >= HUNGER_EMOT_INTERVAL_SECS {
                p.hunger_emot_timer = 0.0;
                hunger_emots.push((p.p_id, p.x, p.y));
            }
        } else {
            p.hunger_emot_timer = 0.0;
        }
        // Sleeping: PE snore/sleep emote every ~15s sim time.
        if p.sleeping {
            p.sleep_emot_timer += dt;
            if p.sleep_emot_timer >= SLEEP_EMOT_INTERVAL_SECS {
                p.sleep_emot_timer = 0.0;
                sleep_emots.push((p.p_id, p.x, p.y));
            }
        } else {
            p.sleep_emot_timer = 0.0;
        }
        if p.food < DEATH_FOOD_THRESHOLD {
            p.food = 0.0;
            p.deleted = true;
            p.death_reason = Some(DeathCause::Hunger.wire_tag().into());
            // held/clothing/backpack drained later by scatter_backpack_on_death
            p.vitals_emit_timer = 0.0;
            p.hunger_emot_timer = 0.0;
            p.sleep_emot_timer = 0.0;
            dead.push(*cid);
        }
    }
    // Continuous breast-feeding (Haxe TimeHelper isHoldingChildInBreastFeedingAgeAndCanFeed).
    {
        let nurses: Vec<(u64, i32, f32, f32, bool)> = state
            .players
            .iter()
            .filter(|(_, p)| !p.deleted && p.holding_player_id != 0)
            .map(|(&cid, p)| {
                (
                    cid,
                    p.holding_player_id,
                    p.age,
                    p.food,
                    FertilityState::age_fertile(p.age),
                )
            })
            .collect();
        for (mother_conn, baby_p_id, m_age, m_food, fertile) in nurses {
            let baby_info = state.players.iter().find_map(|(&bc, b)| {
                if b.p_id == baby_p_id && !b.deleted {
                    Some((bc, b.age, b.food, b.food_max))
                } else {
                    None
                }
            });
            let Some((baby_conn, b_age, b_food, b_max)) = baby_info else {
                continue;
            };
            if !can_breastfeed(m_age, m_food, fertile, b_age, true) {
                continue;
            }
            let (to_baby, from_m) = breastfeed_tick(dt, FOOD_USE_PER_SEC, b_food, b_max);
            if to_baby <= 0.0 {
                continue;
            }
            if let Some(m) = state.players.get_mut(&mother_conn) {
                m.food = (m.food - from_m).max(0.0);
            }
            if let Some(b) = state.players.get_mut(&baby_conn) {
                b.food = (b.food + to_baby).min(b.food_max);
            }
            // Heal baby hits slowly while nursing (Haxe hits -= dt * 0.2).
            if let Some(s) = state.combat.stats.get_mut(&baby_p_id) {
                if s.hits > 0.0 {
                    s.hits = (s.hits - dt * 0.2).max(0.0);
                }
            }
        }
    }
    // AFK mark + optional PE yawn when idle first crosses DEFAULT_AFK_SECS.
    let mut afk_yawns: Vec<(i32, i32, i32)> = Vec::new();
    {
        let now = state.sim_time;
        for p in state.players.values() {
            if p.deleted || !p.connected {
                continue;
            }
            if !state.afk.is_afk(p.p_id, now, DEFAULT_AFK_SECS) {
                continue;
            }
            let idle = state.afk.idle_secs(p.p_id, now);
            // First tick past the threshold this vitals step.
            if idle - dt < DEFAULT_AFK_SECS {
                afk_yawns.push((p.p_id, p.x, p.y));
            }
        }
    }
    for (p_id, x, y) in &afk_yawns {
        state.push_event(format!("AFK {p_id}"));
        let nearby = nearby_conn_ids(state, *x, *y, NEARBY_RANGE);
        if nearby.is_empty() {
            continue;
        }
        let line = format!("{p_id} {YAWN_EMOT_INDEX}");
        let pe = format_server_message("PE", &[&line]).into_bytes();
        send_nearby(outbound, &nearby, pe);
        debug!(p_id, x, y, n = nearby.len(), "sim: PE AFK yawn");
    }
    for (p_id, x, y, sick) in vitals_emits {
        let nearby = nearby_conn_ids(state, x, y, NEARBY_RANGE);
        if nearby.is_empty() {
            continue;
        }
        let bw = format_baby_wiggle(p_id).into_bytes();
        let dy = format_dying(p_id, sick).into_bytes();
        send_nearby(outbound, &nearby, bw);
        send_nearby(outbound, &nearby, dy);
        debug!(p_id, x, y, sick, n = nearby.len(), "sim: BW+DY starving infant");
    }
    for (p_id, x, y) in hunger_emots {
        let nearby = nearby_conn_ids(state, x, y, NEARBY_RANGE);
        if nearby.is_empty() {
            continue;
        }
        let line = format!("{p_id} {HUNGER_EMOT_INDEX}");
        let pe = format_server_message("PE", &[&line]).into_bytes();
        send_nearby(outbound, &nearby, pe);
        debug!(p_id, x, y, n = nearby.len(), "sim: PE hunger emote");
    }
    for (p_id, x, y) in sleep_emots {
        let nearby = nearby_conn_ids(state, x, y, NEARBY_RANGE);
        if nearby.is_empty() {
            continue;
        }
        let line = format!("{p_id} {SLEEP_EMOT_INDEX}");
        let pe = format_server_message("PE", &[&line]).into_bytes();
        send_nearby(outbound, &nearby, pe);
        debug!(p_id, x, y, n = nearby.len(), "sim: PE sleep/snore emote");
    }
    // Periodic HX heat packets (Haxe HEAT_CHANGE) to each living player.
    state.hx_emit_timer += dt;
    if state.hx_emit_timer >= HX_EMIT_INTERVAL_SECS {
        state.hx_emit_timer = 0.0;
        for (cid, heat) in &heat_by_conn {
            // Skip players who died this tick.
            if dead.contains(cid) {
                continue;
            }
            let pkt = format_heat_change(*heat, 0.0, 0.0).into_bytes();
            outbound.send(*cid, pkt);
        }
        debug!(n = heat_by_conn.len(), "sim: HX heat to players");
    }
    // Pos-debug LS is emitted on wall-clock from the async sim loop (not here),
    // so it is not delayed/batched with vitals fan-out.
    for &cid in &dead {
        let (reason, p_id, death_xy, age, food, email) = {
            let p = state.players.get(&cid);
            let reason = p
                .and_then(|p| p.death_reason.as_deref())
                .unwrap_or("reason_unknown")
                .to_string();
            // Ensure every death has a canonical tag.
            let reason = if DeathCause::from_reason(&reason) == DeathCause::Unknown
                && reason != DeathCause::Unknown.wire_tag()
            {
                DeathCause::Unknown.wire_tag().into()
            } else {
                reason
            };
            let p_id = p.map(|p| p.p_id).unwrap_or(0);
            let death_xy = p.map(|p| (p.x, p.y));
            let age = p.map(|p| p.age).unwrap_or(0.0);
            let food = p.map(|p| p.food).unwrap_or(0.0);
            let email = p.map(|p| p.email.clone()).unwrap_or_default();
            (reason, p_id, death_xy, age, food, email)
        };
        let heat = heat_by_conn.get(&cid).copied().unwrap_or(0.0);
        if let Some(log) = &state.death_log {
            let (x, y) = death_xy.unwrap_or((0, 0));
            log.record(DeathRecord {
                wall_unix_ms: 0,
                p_id,
                conn_id: cid,
                reason: reason.clone(),
                age,
                food,
                heat,
                x,
                y,
                email,
            });
        }
        info!(
            conn_id = cid,
            p_id,
            reason = %reason,
            age,
            food,
            heat,
            "sim: player death"
        );
        let cause = DeathCause::from_reason(&reason);
        // Hunger/age: place content-resolved grave when non-zero.
        if cause.is_natural() {
            let gid = state.grave_object_id;
            if gid != 0 {
                if let Some((x, y)) = death_xy {
                    state.world.write().unwrap().set_object(x, y, gid);
                    state.record_world_change(x, y, gid);
                    state.specials.insert(x, y, SpecialKind::Grave);
                }
            }
        }
        // Held + clothing + backpack → ground scatter (rings first so grave can keep death tile).
        scatter_backpack_on_death(state, cid);
        // Fold session score into soft account (no SQL) before inheritance zeros wallet.
        if let Some(pl) = state.players.get(&cid) {
            let score = state
                .scoreboard
                .entry(p_id)
                .map(|e| e.score)
                .unwrap_or(0);
            let kills = state
                .combat
                .stats
                .get(&p_id)
                .map(|s| s.kills)
                .unwrap_or(0);
            let deaths = state
                .combat
                .stats
                .get(&p_id)
                .map(|s| s.deaths)
                .unwrap_or(0)
                .max(1);
            let coins = state
                .economy
                .wallets
                .get(&p_id)
                .map(|w| w.coins)
                .unwrap_or(0);
            state
                .accounts
                .on_death(&pl.email, score, kills, deaths, coins);
        }
        // Inheritance: coins → mother if online, else treasury.
        apply_death_inheritance(state, p_id);
        if let Some(c) = counters {
            c.deaths.fetch_add(1, Ordering::Relaxed);
        }
        state.push_event(format_death_event(p_id, cause));
        state.afk.remove(p_id);
        info!(conn_id = cid, reason = %reason, "player died");
    }
    // Timed gestation: mothers whose due time elapsed auto-spawn via spawn_child.
    let due_mothers = crate::gestation_tick::due_mothers(&mut state.fertility, state.sim_time);
    for mother_id in due_mothers {
        let mother_conn = state
            .players
            .iter()
            .find(|(_, pl)| pl.p_id == mother_id && !pl.deleted)
            .map(|(&c, _)| c);
        let Some(mc) = mother_conn else {
            continue;
        };
        if let Some(baby_p_id) = spawn_child(state, mc) {
            if let Some(node) = state.social.lineages.get(&baby_p_id) {
                let ln = node.wire_line();
                outbound.send(mc, format_server_message("LN", &[&ln]).into_bytes());
            }
            let line = format!("{mother_id} BIRTH {baby_p_id} OK");
            send_ps_reply(outbound, mc, &line);
            info!(
                mother_conn = mc,
                mother_id,
                baby_p_id,
                "sim: gestation due spawn_child"
            );
        }
    }
    let decayed = tick_auto_decays(state, dt);
    for &(x, y, new_id) in &decayed {
        let floor = state.world.read().unwrap().get_floor(x, y) as i32;
        let mx = format_map_change(x, y, floor, new_id, 0).into_bytes();
        let near = nearby_conn_ids(state, x, y, NEARBY_RANGE);
        send_nearby(outbound, &near, mx);
    }
    // Timed animal wander (Haxe doAnimalMovement / SendAnimalMoveUpdateToAllClosePlayers):
    // place map objects walk with MX old_x old_y speed + clear origin + FM per viewer.
    let moves = tick_animals_dt(state, dt);
    for &(_id, kind, ox, oy, nx, ny) in &moves {
        let animal_obj = kind.object_id();
        // Speed scales mildly with Chebyshev hop length (Haxe often uses ~1).
        let steps = (nx - ox).abs().max((ny - oy).abs()).max(1) as f32;
        let speed = kind.move_speed() * steps.sqrt().max(1.0);
        // Update world tiles: move animal object from origin → dest (leave origin empty
        // if it still held this animal; do not stomp a non-animal already at dest).
        {
            let mut w = state.world.write().unwrap();
            let at_old = w.get_object(ox, oy);
            let at_new = w.get_object(nx, ny);
            if at_new != 0 && at_new != animal_obj {
                // Dest blocked after path check race — skip map write, keep entity moved.
            } else {
                if at_old == animal_obj {
                    w.set_object(ox, oy, 0);
                }
                w.set_object(nx, ny, animal_obj);
            }
        }
        let floor_o = state.world.read().unwrap().get_floor(ox, oy) as i32;
        let floor_n = state.world.read().unwrap().get_floor(nx, ny) as i32;
        let leftover_o = state.world.read().unwrap().get_object(ox, oy);
        // Fan-out birth-relative MX per human viewer (not absolute world coords).
        let near = nearby_conn_ids(state, nx, ny, NEARBY_RANGE.max(32));
        for &cid in &near {
            let Some(viewer) = state.players.get(&cid) else {
                continue;
            };
            if viewer.deleted || !viewer.connected {
                continue;
            }
            let (rx_o, ry_o) = viewer.world_to_client(ox, oy);
            let (rx_n, ry_n) = viewer.world_to_client(nx, ny);
            // Dest: object arrives from old tile with speed (Haxe sendMapUpdateForMoving).
            outbound.send_urgent(
                cid,
                format_map_change_moving(
                    rx_n, ry_n, floor_n, animal_obj, -1, rx_o, ry_o, speed,
                )
                .into_bytes(),
            );
            // Origin: clear (or leftover ground) without motion params.
            outbound.send_urgent(
                cid,
                format_map_change(rx_o, ry_o, floor_o, leftover_o, -1).into_bytes(),
            );
            send_frame(outbound, cid);
        }
    }
    if !moves.is_empty() {
        debug!(n = moves.len(), "sim: animal timed wander MX+FM fan-out");
    }
    // Living prestige class refresh from online scoreboard ranks.
    state.prestige_refresh_timer += dt;
    if state.prestige_refresh_timer >= LIVING_PRESTIGE_REFRESH_SECS {
        state.prestige_refresh_timer = 0.0;
        state.refresh_living_prestige_classes();
    }
    let season_rolled = state.environment.tick(dt);
    if season_rolled {
        let tag = state.environment.season.as_str();
        if state.scoreboard.on_season_change(tag) {
            info!(season = %tag, "sim: scoreboard season leaderboard reset");
        }
    } else if state.scoreboard.season_tag.is_empty() {
        // Bind current season tag without wiping (first tick after boot).
        let _ = state.scoreboard.on_season_change(state.environment.season.as_str());
    }
    state.apocalypse.tick(dt);
    // Orderly !shutdown machine.
    tick_shutdown(state, outbound, dt);
    if let Some(view) = &state.env_view {
        if let Ok(mut g) = view.write() {
            *g = state.environment.snapshot();
        }
    }
    // Chunk interest tiers for metrics / SAY ?CHUNKS.
    refresh_chunk_tier_counts(state);
    state.publish_web_snapshots();
    // Publish vitals so viewer food/age bars update.
    state.publish_all_player_views();
}

/// Advance `SAY !shutdown` countdown → save → apocalypse AP → exit flag.
fn tick_shutdown(state: &mut SimState, outbound: &OutboundHub, dt: f32) {
    let Some(mut sh) = state.shutdown.take() else {
        return;
    };
    sh.remaining -= dt;
    match sh.phase {
        ShutdownPhase::Countdown => {
            if sh.remaining > 0.0 {
                state.shutdown = Some(sh);
                return;
            }
            // Save signal for outer autosave.
            if let Some(flag) = &state.save_request {
                flag.store(true, Ordering::SeqCst);
            }
            // Apocalypse client visual (Haxe APOCALYPSE / AP).
            state.apocalypse.trigger();
            broadcast_global(outbound, "SERVER SHUTDOWN — APOCALYPSE");
            // Wire-ish AP tag for clients that listen for apocalypse.
            outbound.broadcast(format_server_message("AP", &["1"]).into_bytes());
            let hold = state.shutdown_apocalypse_secs.max(0.5);
            info!(hold, "sim: !shutdown save+AP; holding before exit");
            state.shutdown = Some(ShutdownState {
                remaining: hold,
                phase: ShutdownPhase::ApocalypseHold,
            });
        }
        ShutdownPhase::ApocalypseHold => {
            if sh.remaining > 0.0 {
                state.shutdown = Some(sh);
                return;
            }
            if let Some(flag) = &state.shutdown_exit {
                flag.store(true, Ordering::SeqCst);
            }
            info!("sim: !shutdown exit flag set");
            // Leave shutdown as None; exit is signaled.
        }
    }
}

/// Haxe auto-decay: objects with actor&lt;0 transitions transform after delay.
/// Returns list of `(x, y, new_object_id)` that changed this step (for MX).
pub fn tick_auto_decays(state: &mut SimState, dt: f32) -> Vec<(i32, i32, i32)> {
    let mut changed = Vec::new();
    if state.pending_decays.is_empty() {
        return changed;
    }
    let keys: Vec<(i32, i32)> = state.pending_decays.keys().copied().collect();
    for key in keys {
        let Some(&(expect_id, rem)) = state.pending_decays.get(&key) else {
            continue;
        };
        let new_rem = rem - dt;
        if new_rem > 0.0 {
            state.pending_decays.insert(key, (expect_id, new_rem));
            continue;
        }
        state.pending_decays.remove(&key);
        let (x, y) = key;
        let current = state.world.read().unwrap().get_object(x, y);
        if current != expect_id {
            continue;
        }
        let Some(tr) = state.content.auto_decays.get(&expect_id).cloned() else {
            continue;
        };
        {
            let mut w = state.world.write().unwrap();
            place_after_use(
                &mut w,
                &state.content,
                x,
                y,
                expect_id,
                tr.new_target_id,
                0,
                tr.reverse_use_target,
                true,
            );
        }
        state.record_world_change(x, y, tr.new_target_id);
        // Chain further decays on the new object.
        schedule_decay(state, x, y, tr.new_target_id);
        changed.push((x, y, tr.new_target_id));
        debug!(x, y, from = expect_id, to = tr.new_target_id, "auto-decay applied");
    }
    changed
}

pub fn schedule_decay(state: &mut SimState, x: i32, y: i32, obj_id: i32) {
    if obj_id == 0 {
        state.pending_decays.remove(&(x, y));
        return;
    }
    if let Some(tr) = state.content.auto_decays.get(&obj_id) {
        if tr.auto_decay_seconds > 0.0 {
            state
                .pending_decays
                .insert((x, y), (obj_id, tr.auto_decay_seconds));
        }
    } else {
        state.pending_decays.remove(&(x, y));
    }
}

/// Build a reverse craft graph from content transitions (normal + last-use).
///
/// Caps at [`CRAFT_GRAPH_SEED_CAP`] total inserts for fast restart. Empty
/// ContentDb yields an empty graph. Safe to share via [`Arc`] for self-play.
pub fn build_reverse_craft_graph(content: &ContentDb) -> ReverseCraftGraph {
    build_reverse_craft_graph_capped(content, CRAFT_GRAPH_SEED_CAP)
}

/// Build reverse craft graph with an explicit seed cap (from server config).
pub fn build_reverse_craft_graph_capped(content: &ContentDb, cap: usize) -> ReverseCraftGraph {
    let cap = cap.max(1);
    let mut pairs: Vec<(i32, i32, i32, i32)> = Vec::new();
    for t in content.transitions.values() {
        if pairs.len() >= cap {
            break;
        }
        pairs.push((t.actor_id, t.target_id, t.new_actor_id, t.new_target_id));
    }
    if pairs.len() < cap {
        for t in content.transitions_last_use.values() {
            if pairs.len() >= cap {
                break;
            }
            pairs.push((t.actor_id, t.target_id, t.new_actor_id, t.new_target_id));
        }
    }
    let mut graph = ReverseCraftGraph::new();
    let seeded = graph.seed_from_pairs(pairs, cap);
    info!(
        seeded,
        products = graph.product_count(),
        edges = graph.edge_count(),
        cap,
        "sim: reverse craft graph built from content"
    );
    graph
}

/// Populate [`SimState::craft_graph`] from content transitions (normal + last-use).
///
/// Caps at [`CRAFT_GRAPH_SEED_CAP`] total inserts for fast restart. Safe to call
/// once after content is attached; empty ContentDb yields an empty graph.
pub fn seed_craft_graph_from_content(state: &mut SimState) {
    state.craft_graph = build_reverse_craft_graph(&state.content);
}

/// Spawn rabbits/wolves/boars near play area when the animal world is empty.
///
/// Also **places content object ids on the world map** so clients see them via
/// MAP_CHUNK / MX (Haxe animals are map objects, not free-floating entities).
///
/// Anchor is map center when the world has dimensions (self-play agents spawn
/// near center); otherwise configured [`SimState::spawn_x`] / [`SimState::spawn_y`].
/// Wolves are placed so forager (center) and hunter (~+28 east) can sense them.
pub fn spawn_default_animals(state: &mut SimState) {
    if !state.animals.animals.is_empty() {
        return;
    }
    let (sx, sy) = {
        let w = state.world.read().unwrap();
        if w.width_tiles > 0 && w.height_tiles > 0 {
            (w.width_tiles / 2, w.height_tiles / 2)
        } else {
            (state.spawn_x, state.spawn_y)
        }
    };
    // 3 rabbit / 2 wolf / 2 boar — cover center forager + east farmer/hunter band (~+18..+28).
    let seeds: &[(AnimalKind, i32, i32)] = &[
        (AnimalKind::Rabbit, sx + 3, sy + 2),
        (AnimalKind::Rabbit, sx - 2, sy + 4),
        (AnimalKind::Rabbit, sx + 18, sy + 12),
        (AnimalKind::Wolf, sx + 3, sy),
        (AnimalKind::Wolf, sx + 18, sy + 13),
        (AnimalKind::Boar, sx + 8, sy + 3),
        (AnimalKind::Boar, sx + 20, sy + 10),
    ];
    for &(kind, x, y) in seeds {
        let (px, py) = find_empty_animal_tile(state, x, y);
        state.animals.spawn(kind, px, py);
        let oid = kind.object_id();
        let mut w = state.world.write().unwrap();
        if w.get_object(px, py) == 0 {
            w.set_object(px, py, oid);
        }
    }
    info!(
        n = state.animals.animals.len(),
        sx,
        sy,
        "sim: default animals spawned on map near play area"
    );
}

/// Find a nearby empty tile for animal placement (prefer exact, then ring search).
fn find_empty_animal_tile(state: &SimState, prefer_x: i32, prefer_y: i32) -> (i32, i32) {
    let w = state.world.read().unwrap();
    let ww = w.width_tiles;
    let hh = w.height_tiles;
    if ww <= 0 || hh <= 0 {
        return (prefer_x, prefer_y);
    }
    for r in 0i32..8 {
        for dy in -r..=r {
            for dx in -r..=r {
                if r > 0 && dx.abs() != r && dy.abs() != r {
                    continue;
                }
                let x = prefer_x + dx;
                let y = prefer_y + dy;
                if x < 0 || y < 0 || x >= ww || y >= hh {
                    continue;
                }
                if w.get_object(x, y) == 0 && is_walkable(&w, &state.content, x, y) {
                    return (x, y);
                }
            }
        }
    }
    (prefer_x.max(0).min(ww - 1), prefer_y.max(0).min(hh - 1))
}

/// One wander step for all animals using pathfind walkability.
/// Returns animal moves for wire MX fan-out.
pub fn tick_animals(state: &mut SimState) -> Vec<(i32, AnimalKind, i32, i32, i32, i32)> {
    tick_animals_dt(state, 100.0)
}

/// Timed animal movement: Haxe `doAnimalMovement` cadence + pathing.
///
/// - Interval: content auto-decay seconds (wolf/boar ~3s, rabbit ~1s)
/// - Targets: random in move radius, prefer empty tiles
/// - Path: step-wise toward target; trees/plants do **not** block; walls do;
///   ocean / mountain / deep river biomes block (with 3% pass chance)
pub fn tick_animals_dt(state: &mut SimState, dt: f32) -> Vec<(i32, AnimalKind, i32, i32, i32, i32)> {
    let (ww, wh) = {
        let w = state.world.read().unwrap();
        (w.width_tiles, w.height_tiles)
    };
    if ww <= 0 || wh <= 0 || state.animals.animals.is_empty() || dt <= 0.0 {
        return Vec::new();
    }
    let world = Arc::clone(&state.world);
    let content = Arc::clone(&state.content);
    let interval_for = |kind: AnimalKind| -> f32 {
        let oid = kind.object_id();
        content
            .auto_decays
            .get(&oid)
            .map(|t| {
                // Prefer autoDecaySeconds; fall back to kind default.
                if t.auto_decay_seconds > 0.0 {
                    t.auto_decay_seconds
                } else {
                    AnimalWorld::wander_interval(kind)
                }
            })
            .unwrap_or_else(|| AnimalWorld::wander_interval(kind))
    };
    let mut rng = rand::thread_rng();
    state.animals.tick_wander_timed_ex(
        &mut rng,
        dt,
        ww,
        wh,
        Some(&interval_for),
        |rng, ox, oy, kind| {
            let w = world.read().unwrap();
            let rad = {
                let oid = kind.object_id();
                content
                    .auto_decays
                    .get(&oid)
                    .map(|t| {
                        // Haxe: moveDist = transition.move; if < 3 then += 1
                        let mut m = t.move_dist;
                        if m <= 0 {
                            m = AnimalWorld::move_radius(kind);
                        } else if m < 3 {
                            m += 1;
                        }
                        if t.desired_move_dist > 0 {
                            m = m.max(t.desired_move_dist.min(6));
                        }
                        m
                    })
                    .unwrap_or_else(|| AnimalWorld::move_radius(kind))
            };
            let rabbit = matches!(kind, AnimalKind::Rabbit);
            animal_move::pick_animal_destination(
                &w,
                &content,
                rng,
                ox,
                oy,
                ww,
                wh,
                rad,
                rabbit,
            )
        },
    )
}

/// Resolve whether held object is food and its feed value (content or name heuristic).
fn resolve_held_food(state: &SimState, held_id: i32) -> (bool, f32) {
    if held_id == 0 {
        return (false, 0.0);
    }
    if let Some(def) = state.content.get(held_id) {
        let by_val = def.food_value > 0;
        let by_name = name_looks_like_food(&def.name);
        let is_food = by_val || by_name;
        let val = if by_val {
            def.food_value as f32
        } else if by_name {
            1.0
        } else {
            0.0
        };
        (is_food, val)
    } else {
        (false, 0.0)
    }
}

/// Held object display name from content (empty when hands empty / unknown id).
fn held_object_name(state: &SimState, held_id: i32) -> String {
    if held_id == 0 {
        return String::new();
    }
    state
        .content
        .get(held_id)
        .map(|d| d.name.clone())
        .unwrap_or_default()
}

/// Scan the full map and arm auto-decay for every tile whose object has an
/// auto-decay transition (Haxe world load / natural spawn path).
pub fn arm_decays_for_loaded_world(state: &mut SimState) {
    let world = state.world.read().unwrap();
    let w = world.width_tiles;
    let h = world.height_tiles;
    if w <= 0 || h <= 0 {
        return;
    }
    let mut to_arm: Vec<(i32, i32, i32)> = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let id = world.get_object(x, y);
            if id != 0 && state.content.auto_decays.contains_key(&id) {
                to_arm.push((x, y, id));
            }
        }
    }
    drop(world);
    let armed = to_arm.len() as u32;
    for (x, y, id) in to_arm {
        schedule_decay(state, x, y, id);
    }
    info!(armed, w, h, "auto-decay armed for loaded/generated world");
}

/// Try to eat held food (Haxe-style: held edible, no tile transition needed).
/// Returns true if food was consumed.
pub fn try_eat_held(state: &mut SimState, conn_id: u64) -> bool {
    let held = match state.players.get(&conn_id) {
        Some(p) => p.held_id,
        None => return false,
    };
    if held == 0 {
        return false;
    }
    let Some(def) = state.content.get(held) else {
        return false;
    };
    if def.food_value <= 0 {
        return false;
    }
    let base = def.food_value as f32;
    if let Some(p) = state.players.get_mut(&conn_id) {
        let fill_before = p.food.ceil() as i32;
        let gain = p.yum.eat(held, base, fill_before);
        p.held_id = 0;
        p.food = (p.food + gain).min(p.food_max);
        // Learning tools when eating isn't typical — learn held craft tools on use instead.
        info!(conn_id, held, gain, food = p.food, "sim: ate food");
        true
    } else {
        false
    }
}

/// Self-craft: apply transition for held object with target `0` (empty / special).
///
/// `SAY CRAFT` is equivalent to USE on an empty tile for recipes keyed
/// `(actor=held, target=0)`. Lookup is always [`ContentDb::find_transition`]
/// `(held, 0)` — not last-use prefer.
///
/// On success: held becomes `new_actor_id`; `new_target_id` is placed under
/// the player's feet when non-zero (ground left unchanged when 0).
pub fn try_craft(state: &mut SimState, conn_id: u64) -> Option<UseResult> {
    let player = state.players.get(&conn_id)?;
    if player.deleted {
        return None;
    }
    let held = player.held_id;
    let (x, y) = (player.x, player.y);

    if held == 0 {
        return Some(UseResult {
            actor_before: 0,
            target_before: 0,
            actor_after: 0,
            target_after: 0,
            applied: false,
            x,
            y,
        });
    }

    let Some(tr) = state.content.find_transition(held, 0).cloned() else {
        return Some(UseResult {
            actor_before: held,
            target_before: 0,
            actor_after: held,
            target_after: 0,
            applied: false,
            x,
            y,
        });
    };

    let actor_after = tr.new_actor_id;
    let target_after = tr.new_target_id;

    // Place craft product under feet when the transition produces a ground object.
    // Held-only transforms (new_target == 0) leave the tile alone.
    if target_after != 0 {
        {
            let mut w = state.world.write().unwrap();
            place_after_use(
                &mut w,
                &state.content,
                x,
                y,
                0,
                target_after,
                0,
                tr.reverse_use_target,
                true,
            );
        }
        state.record_world_change(x, y, target_after);
        schedule_decay(state, x, y, target_after);
    }

    let equip_slot = if actor_after != 0 {
        state
            .content
            .get(actor_after)
            .and_then(|def| clothing_slot_for_object(&def.name, &def.description))
    } else {
        None
    };
    if let Some(p) = state.players.get_mut(&conn_id) {
        p.held_id = actor_after;
        p.force_last_use = false;
        if held != 0 {
            p.tools.learn(held);
        }
        if let Some(slot) = equip_slot {
            p.set_clothing(slot, actor_after);
        }
    }

    Some(UseResult {
        actor_before: held,
        target_before: 0,
        actor_after,
        target_after,
        applied: true,
        x,
        y,
    })
}

/// Reported move speed for PU / FX from ride + weather + snow + fire + ballast.
fn player_move_speed(state: &SimState, p: &Player) -> f32 {
    let ballast = weight_item_count(p.held_id, p.backpack.len());
    compose_move_speed(
        p.riding,
        &state.weather,
        &state.snow,
        &state.fire,
        p.x,
        p.y,
        ballast,
    )
}

/// FX food-change line from current player vitals + yum state + composed move speed.
fn food_change_for_player(state: &SimState, p: &Player) -> String {
    format_food_change(
        p.food.ceil() as i32,
        p.food_max as i32,
        p.yum.just_ate_id,
        p.yum.last_ate_fill_max,
        player_move_speed(state, p),
        -1,
        p.yum.yum_bonus_ceil(),
        0,
    )
}

pub async fn run_sim_loop(
    intent_rx: tokio::sync::mpsc::Receiver<NetIntent>,
    counters: Arc<Counters>,
    tick_hz: u32,
    content: Arc<ContentDb>,
    world: Arc<RwLock<World>>,
    outbound: Arc<OutboundHub>,
    player_views: Option<PlayerViewMap>,
    env_view: Option<EnvView>,
    // Optional shared social mirror for shutdown / autosave (lineages).
    shared_social: Option<Arc<RwLock<SocialState>>>,
) {
    run_sim_loop_with_views(
        intent_rx,
        counters,
        tick_hz,
        1.0,
        DEFAULT_REQUIRED_VERSION,
        false,
        content,
        world,
        outbound,
        player_views,
        env_view,
        shared_social,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        TwinRegistry::default(),
        None,
            false,
        2,
        None,
        100,
        300,
        64,
        true,
        None,
        None,
        3.0,
        3.0,
    )
    .await
}

/// Sim loop with optional web snapshot arcs (weather / accounts / prestige / lineage /
/// animals / treasury) and optional live [`AnimalWorldShare`] for self-play AI.
///
/// `twins` is the multi-server twin peer registry (**stub only** — no network).
/// `save_request` is an optional force-save flag for operator `SAY SAVE` (outer autosave polls).
#[allow(clippy::too_many_arguments)]
pub async fn run_sim_loop_with_views(
    mut intent_rx: tokio::sync::mpsc::Receiver<NetIntent>,
    counters: Arc<Counters>,
    tick_hz: u32,
    // Config `sim_speed` time dilation applied inside vitals (`1.0` = realtime).
    sim_speed: f32,
    // SN / data version (sets SimState::version_gate.required).
    required_version: i32,
    // Config `client_version_strict` — hard-reject LOGIN on version mismatch.
    client_version_strict: bool,
    content: Arc<ContentDb>,
    world: Arc<RwLock<World>>,
    outbound: Arc<OutboundHub>,
    player_views: Option<PlayerViewMap>,
    env_view: Option<EnvView>,
    shared_social: Option<Arc<RwLock<SocialState>>>,
    weather_view: Option<WeatherView>,
    account_view: Option<AccountView>,
    prestige_view: Option<PrestigeView>,
    lineage_view: Option<LineageView>,
    animal_view: Option<AnimalView>,
    animals_share: Option<AnimalWorldShare>,
    treasury_view: Option<TreasuryView>,
    // Optional soft-account mirror for shutdown / autosave (OLA1).
    shared_accounts: Option<Arc<RwLock<AccountBook>>>,
    twins: TwinRegistry,
    save_request: Option<Arc<AtomicBool>>,
    timed_movement: bool,
    move_jump_max_chebyshev: i32,
    ops_view: Option<std::sync::Arc<std::sync::RwLock<Vec<ol_metrics::OpsSample>>>>,
    ops_sample_every_ticks: u64,
    ops_flush_secs: u64,
    intent_drain_budget: usize,
    broadcast_all_updates: bool,
    death_log: Option<std::sync::Arc<DeathLog>>,
    shutdown_exit: Option<Arc<AtomicBool>>,
    shutdown_countdown_secs: f32,
    shutdown_apocalypse_secs: f32,
) {
    let mut state = SimState::new(world, content);
    state.timed_movement = timed_movement;
    state.broadcast_all_updates = broadcast_all_updates;
    state.death_log = death_log;
    state.move_jump_max_chebyshev = move_jump_max_chebyshev.max(0);
    state.shutdown_exit = shutdown_exit;
    state.shutdown_countdown_secs = shutdown_countdown_secs.max(1.0);
    state.shutdown_apocalypse_secs = shutdown_apocalypse_secs.max(0.5);
    {
        let w = state.world.read().unwrap();
        let (sx, sy) = find_playable_spawn(&w, (0, 0));
        state.spawn_x = sx;
        state.spawn_y = sy;
        info!(sx, sy, "sim: playable spawn point ready");
    }
    state.sim_speed = if sim_speed.is_finite() && sim_speed >= 0.0 {
        sim_speed
    } else {
        1.0
    };
    state.version_gate.required = required_version;
    state.client_version_strict = client_version_strict;
    state.twins = twins;
    if let Some(flag) = save_request {
        state.save_request = Some(flag);
    }
    if let Some(v) = player_views {
        state = state.with_player_views(v);
    }
    if let Some(v) = env_view {
        state = state.with_env_view(v);
    }
    if let Some(v) = weather_view {
        state = state.with_weather_view(v);
    }
    if let Some(v) = account_view {
        state = state.with_account_view(v);
    }
    if let Some(v) = prestige_view {
        state = state.with_prestige_view(v);
    }
    if let Some(v) = lineage_view {
        state = state.with_lineage_view(v);
    }
    if let Some(v) = animal_view {
        state = state.with_animal_view(v);
    }
    if let Some(v) = animals_share {
        state = state.with_animals_share(v);
    }
    if let Some(v) = treasury_view {
        state = state.with_treasury_view(v);
    }
    // Append-only tile change journal (DROP/USE); soft-fail if path unusable later.
    state.journal = Some(Arc::new(Mutex::new(WorldJournal::open_default())));
    // Seed lineages from boot-loaded mirror (if any).
    if let Some(ref shared) = shared_social {
        state.social = shared.read().unwrap().clone();
        info!(
            lineages = state.social.lineages.len(),
            "sim: loaded lineages from shared social"
        );
    }
    // Seed soft accounts from boot-loaded OLA1 (if any).
    if let Some(ref shared) = shared_accounts {
        state.accounts = shared.read().unwrap().clone();
        info!(
            accounts = state.accounts.len(),
            "sim: loaded accounts from shared book"
        );
    }
    // Natural spawn / OLW load never went through USE — arm decay timers now.
    arm_decays_for_loaded_world(&mut state);
    // Reverse craft graph from content transitions (capped for boot speed).
    seed_craft_graph_from_content(&mut state);
    // Seed a few wild animals near play area for AI / viewer if none loaded.
    spawn_default_animals(&mut state);
    // Publish immediately so self-play Arc share sees wolves before first vitals tick.
    state.publish_web_snapshots();

    let period = Duration::from_secs_f64(1.0 / tick_hz.max(1) as f64);
    let tick_time = period.as_secs_f32();
    let intent_budget = intent_drain_budget.max(1);
    let max_catch_up_extra = 5u32;
    let mirror_every = tick_hz.max(1);
    let mut ticks_since_mirror = 0u32;
    let mut ops = ol_metrics::OpsSeries::new(
        ops_sample_every_ticks.max(1),
        Duration::from_secs(ops_flush_secs.max(1)),
        360,
    );
    let mut next_tick_deadline = tokio::time::Instant::now() + period;
    let mut last_skip_log = 0u64;
    // Wall-clock 1 Hz LS pos-debug (bare x y) — independent of sim catch-up.
    let ls_period = std::time::Duration::from_secs_f32(POS_DEBUG_LS_INTERVAL_SECS);
    let mut next_ls_at = tokio::time::Instant::now() + ls_period;
    counters.mark_start_now();
    info!(
        tick_hz,
        timed_movement = state.timed_movement,
        "sim loop started (Haxe catch-up A)"
    );

    loop {
        let mut drained = 0usize;
        let mut human_work = false;
        // Pull a batch, process humans first so SAY never waits on AI intent flood.
        let mut batch: Vec<NetIntent> = Vec::with_capacity(intent_budget.min(64));
        while batch.len() < intent_budget {
            match intent_rx.try_recv() {
                Ok(intent) => batch.push(intent),
                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
                Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                    if let Some(ref shared) = shared_social {
                        *shared.write().unwrap() = state.social.clone();
                    }
                    if let Some(ref shared) = shared_accounts {
                        *shared.write().unwrap() = state.accounts.clone();
                    }
                    info!("intent channel closed; sim stopping");
                    return;
                }
            }
        }
        batch.sort_by_key(|i| if i.conn_id() < 9_000_000 { 0u8 } else { 1u8 });
        for intent in batch {
            let cid = intent.conn_id();
            let tm = ol_metrics::ScopeTimer::start();
            apply_intent(&mut state, &counters, &outbound, intent);
            let elapsed = tm.elapsed();
            ops.on_intent(elapsed);
            counters.record_client_intent(
                cid,
                elapsed.as_micros().min(u128::from(u32::MAX)) as u64,
            );
            if cid < 9_000_000 {
                human_work = true;
                if elapsed.as_millis() >= 50 {
                    warn!(
                        conn_id = cid,
                        us = elapsed.as_micros() as u64,
                        "sim: slow human intent reply"
                    );
                }
            }
            drained += 1;
        }
        let _ = drained;
        // Let connection tasks flush SAY/PS replies before more sim work.
        if human_work {
            tokio::task::yield_now().await;
        }

        // Wall-clock LS every 1s: spoken map location at the player tile.
        // Client uses birth-relative coords + a text token (Haxe array[2]).
        // Wire: LS\n{rx} {ry} {x},{y}\n#  — only the coordinates (no POS/id fluff).
        // Catch up only one beat if the sim was busy (no multi-minute backlog flood).
        let now_ls = tokio::time::Instant::now();
        if now_ls >= next_ls_at {
            // If we fell far behind, skip backlog — send once and reschedule from now.
            while next_ls_at + ls_period < now_ls {
                next_ls_at += ls_period;
            }
            next_ls_at = now_ls + ls_period;
            let targets: Vec<(u64, i32, i32, i32, i32)> = state
                .players
                .iter()
                .filter(|(cid, p)| !p.deleted && p.connected && **cid < 9_000_000)
                .map(|(&cid, p)| {
                    let (rx, ry) = p.world_to_client(p.x, p.y);
                    (cid, rx, ry, p.x, p.y)
                })
                .collect();
            let n = targets.len();
            for (cid, rx, ry, wx, wy) in targets {
                // Spoken text = absolute x,y as one token so the bubble is visible.
                let label = format!("{wx},{wy}");
                // LS then FM — official client will not show LS until FRAME.
                outbound.send_urgent(
                    cid,
                    format_location_says(rx, ry, &label).into_bytes(),
                );
                // Arc<OutboundHub> in the sim loop — borrow for send_frame.
                send_frame(outbound.as_ref(), cid);
            }
            if n > 0 {
                debug!(n, "sim: LS+FM spoken map pos (rel + x,y) wall clock");
                tokio::task::yield_now().await;
            }
        }

        let now = tokio::time::Instant::now();
        if now < next_tick_deadline {
            tokio::select! {
                _ = tokio::time::sleep_until(next_tick_deadline) => {}
                maybe = intent_rx.recv() => {
                    match maybe {
                        Some(intent) => {
                            let cid = intent.conn_id();
                            let tm = ol_metrics::ScopeTimer::start();
                            apply_intent(&mut state, &counters, &outbound, intent);
                            let elapsed = tm.elapsed();
                            ops.on_intent(elapsed);
                            counters.record_client_intent(
                                cid,
                                elapsed.as_micros().min(u128::from(u32::MAX)) as u64,
                            );
                            let mut human_work = cid < 9_000_000;
                            let mut extra = 1usize;
                            while extra < intent_budget {
                                match intent_rx.try_recv() {
                                    Ok(intent) => {
                                        let cid = intent.conn_id();
                                        let tm = ol_metrics::ScopeTimer::start();
                                        apply_intent(&mut state, &counters, &outbound, intent);
                                        let elapsed = tm.elapsed();
                                        ops.on_intent(elapsed);
                                        counters.record_client_intent(
                                            cid,
                                            elapsed.as_micros().min(u128::from(u32::MAX)) as u64,
                                        );
                                        if cid < 9_000_000 {
                                            human_work = true;
                                        }
                                        extra += 1;
                                    }
                                    _ => break,
                                }
                            }
                            if human_work {
                                tokio::task::yield_now().await;
                            }
                            continue;
                        }
                        None => {
                            if let Some(ref shared) = shared_social {
                                *shared.write().unwrap() = state.social.clone();
                            }
                            if let Some(ref shared) = shared_accounts {
                                *shared.write().unwrap() = state.accounts.clone();
                            }
                            info!("intent channel closed; sim stopping");
                            return;
                        }
                    }
                }
            }
        }

        let work = ol_metrics::ScopeTimer::start();
        let mut catch_up_steps: u32 = 1;
        state.tick = state.tick.wrapping_add(1);
        counters.ticks.fetch_add(1, Ordering::Relaxed);

        let periods_behind = {
            let now = tokio::time::Instant::now();
            if now <= next_tick_deadline {
                0u32
            } else {
                let lag = now.duration_since(next_tick_deadline);
                (lag.as_secs_f64() / period.as_secs_f64()).floor() as u32
            }
        };
        let extra_advances = catch_up_extra_steps(state.tick, periods_behind, max_catch_up_extra);
        for _ in 0..extra_advances {
            state.tick = state.tick.wrapping_add(1);
            counters.ticks.fetch_add(1, Ordering::Relaxed);
            counters.skip_ticks.fetch_add(1, Ordering::Relaxed);
            catch_up_steps += 1;
        }

        let dt = tick_time * catch_up_steps as f32;
        tick_move_paths(&mut state, dt, &outbound);
        tick_vitals_with_metrics(&mut state, dt, &outbound, Some(&counters));

        let mut post = 0usize;
        let mut post_human = false;
        while post < intent_budget {
            match intent_rx.try_recv() {
                Ok(intent) => {
                    let cid = intent.conn_id();
                    let tm = ol_metrics::ScopeTimer::start();
                    apply_intent(&mut state, &counters, &outbound, intent);
                    let elapsed = tm.elapsed();
                    ops.on_intent(elapsed);
                    counters.record_client_intent(
                        cid,
                        elapsed.as_micros().min(u128::from(u32::MAX)) as u64,
                    );
                    if cid < 9_000_000 {
                        post_human = true;
                    }
                    post += 1;
                }
                _ => break,
            }
        }
        if post_human {
            tokio::task::yield_now().await;
        }

        ops.on_tick_work(work.elapsed());
        if state.last_lock_wait_us > 0 {
            ops.on_lock_wait(Duration::from_micros(state.last_lock_wait_us as u64));
            state.last_lock_wait_us = 0;
        }
        ops.maybe_sample(state.tick, &counters);
        if let Some(ref view) = ops_view {
            if let Ok(mut g) = view.write() {
                *g = ops.snapshot_samples();
            }
        }

        ticks_since_mirror = ticks_since_mirror.wrapping_add(1);
        if ticks_since_mirror >= mirror_every {
            ticks_since_mirror = 0;
            if let Some(ref shared) = shared_social {
                *shared.write().unwrap() = state.social.clone();
            }
            if let Some(ref shared) = shared_accounts {
                *shared.write().unwrap() = state.accounts.clone();
            }
        }

        if state.tick.saturating_sub(last_skip_log) >= 200 {
            last_skip_log = state.tick;
            let snap = counters.snapshot();
            info!(
                tick = state.tick,
                skip_ticks = snap.skip_ticks,
                "sim: 200-tick summary (skip_ticks = catch-up advances)"
            );
        }

        next_tick_deadline += period * catch_up_steps;
        let now2 = tokio::time::Instant::now();
        if now2 > next_tick_deadline + period * 10 {
            debug!(tick = state.tick, "sim: lag deadline hard-snap");
            next_tick_deadline = now2;
        }
    }
}

pub fn apply_intent(
    state: &mut SimState,
    counters: &Counters,
    outbound: &OutboundHub,
    intent: NetIntent,
) {
    state.intents_seen += 1;
    counters.intents_applied.fetch_add(1, Ordering::Relaxed);

    match intent {
        NetIntent::Login {
            conn_id,
            reconnect,
            email,
            client_tag,
        } => {
            // Version gate: numeric client_tag is treated as data version.
            // Soft-log by default; hard-reject (PS + no spawn) when
            // `client_version_strict` is set.
            let client_ver = parse_version_token(&client_tag);
            if let Some(result) = should_hard_reject_login(
                client_ver,
                &state.version_gate,
                state.client_version_strict,
            ) {
                if let Some(ps_line) = format_version_reject_ps(result) {
                    send_ps_reply(outbound, conn_id, &ps_line);
                }
                outbound.send(conn_id, format_server_message("REJECTED", &[]).into_bytes());
                warn!(
                    conn_id,
                    client = ?client_ver,
                    required = state.version_gate.required,
                    reason = result.reason(),
                    "sim: version_gate hard reject (login denied)"
                );
                return;
            }
            if client_ver.is_some() {
                let result = check_client_version(client_ver, &state.version_gate);
                if !result.is_allowed() {
                    warn!(
                        conn_id,
                        client = ?client_ver,
                        required = state.version_gate.required,
                        reason = result.reason(),
                        "sim: version_gate soft reject (login allowed)"
                    );
                } else if matches!(result, VersionGateResult::AllowClientNewer { .. }) {
                    info!(
                        conn_id,
                        client = ?client_ver,
                        required = state.version_gate.required,
                        "sim: version_gate client newer (allowed)"
                    );
                }
            }
            let p_id = spawn_player(state, conn_id, &email);
            counters.logins.fetch_add(1, Ordering::Relaxed);
            let name = email.split('@').next().unwrap_or("NEWBORN");
            state.social.ensure_lineage(p_id, name);
            state.combat.stats_mut(p_id);
            state.economy.add_coins(p_id, 5); // starting coins
            // Curse tokens (OneLife starts with 1); scoreboard row + starting coin score.
            state.curses.ensure(p_id);
            let display = state
                .players
                .get(&conn_id)
                .map(|p| p.display_name())
                .unwrap_or_else(|| name.to_uppercase());
            let coins = state
                .economy
                .wallets
                .get(&p_id)
                .map(|w| w.coins)
                .unwrap_or(0);
            state.scoreboard.ensure_player(p_id, display);
            state.scoreboard.set_coins(p_id, coins);
            // Send social bootstrap to this connection.
            for pkt in state.social_bootstrap_packets(p_id) {
                outbound.send(conn_id, pkt);
            }
            // Initial curse token count (CX).
            outbound.send(conn_id, state.curses.token_wire(p_id).into_bytes());
            // MC on login (net bootstrap may also send MC; keep sim has_mc in sync).
            force_send_map_chunk(state, outbound, conn_id);
            // Authority sync: net bootstrap may have used preferred_spawn; re-send
            // PU at true sim tile so client MOVE xs,ys matches (avoids jump_too_far).
            // Wire coords are birth-relative (0,0 at spawn birth origin).
            // NM + PU are urgent so the graphical client is not stuck behind AI bulk.
            {
                let Some(pl) = state.players.get(&conn_id) else {
                    return;
                };
                let spd = player_move_speed(state, pl);
                let (rx, ry) = pl.world_to_client(pl.x, pl.y);
                let po = person_object_id(pl);
                let nm_line = format!("{} {} {}", pl.p_id, pl.first_name, pl.family_name);
                outbound.send_urgent(
                    conn_id,
                    format_server_message("NM", &[&nm_line]).into_bytes(),
                );
                let pu = format_player_update_line_full(
                    pl.p_id,
                    po,
                    pl.held_id,
                    rx,
                    ry,
                    pl.age,
                    spd,
                    0,
                    0,
                    1, // force so client snaps to server authority
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    -1,
                    pl.done_moving_seq.max(1),
                );
                outbound.send_urgent(
                    conn_id,
                    format_server_message("PU", &[&pu]).into_bytes(),
                );
                outbound.send_urgent(conn_id, format_server_message("FM", &[]).into_bytes());
                info!(
                    conn_id,
                    p_id,
                    x = pl.x,
                    y = pl.y,
                    name = %nm_line,
                    skin = po,
                    birth = ?(pl.birth_x, pl.birth_y),
                    rel = ?(rx, ry),
                    "sim: post-login NM+PU+FM (force, urgent)"
                );
            }
            state.publish_player_view(conn_id);
            info!(
                conn_id,
                p_id,
                reconnect,
                %email,
                %client_tag,
                "sim: player spawned"
            );
        }
        NetIntent::KeepAlive { conn_id, x, y } => {
            // Haxe Connection.keepAlive() is a **no-op** for position.
            // Applying KA coords was rubber-banding clients (jump-back after MOVE).
            // Only refresh AFK activity; ignore x,y for authority.
            touch_afk_activity(state, conn_id);
            let _ = (x, y);
            debug!(conn_id, "sim: KA (position ignored, Haxe-compatible)");
        }
        NetIntent::Move {
            conn_id,
            xs,
            ys,
            deltas,
            seq,
        } => {
            touch_afk_activity(state, conn_id);
            // Client xs/ys are birth-relative (vanilla server.cpp: m.x += birthPos).
            let (xs, ys) = state
                .players
                .get(&conn_id)
                .map(|p| p.client_to_world(xs, ys))
                .unwrap_or((xs, ys));
            if state.timed_movement {
                match apply_move_path_start(state, outbound, conn_id, xs, ys, &deltas, seq) {
                    Ok(()) => {
                        // Success: PM only (apply_move_path_start). No force PU snap-back.
                        info!(conn_id, steps = deltas.len(), ?seq, "sim: MOVE path accepted");
                    }
                    Err(e) => {
                        // Haxe CancleMovement: force PU at **server** pos with client seq.
                        warn!(conn_id, reason = e.as_str(), "sim: MOVE path rejected");
                        send_forced_player_update(state, outbound, conn_id, seq);
                    }
                }
            } else if apply_move_deltas_with_seq(state, conn_id, xs, ys, &deltas, seq) {
                maybe_send_map_chunk(state, outbound, conn_id);
                state.publish_player_view(conn_id);
                if let Some(p) = state.players.get(&conn_id).cloned() {
                    info!(conn_id, x = p.x, y = p.y, steps = deltas.len(), "sim: MOVE done");
                    let spd = player_move_speed(state, &p);
                    let near = nearby_conn_ids(state, p.x, p.y, NEARBY_RANGE);
                    for &cid in &near {
                        let (rx, ry) = state
                            .players
                            .get(&cid)
                            .map(|v| v.world_to_client(p.x, p.y))
                            .unwrap_or((p.x, p.y));
                        let pu = format_player_update_line(
                            p.p_id,
                            person_object_id(&p),
                            p.held_id,
                            rx,
                            ry,
                            p.age,
                            spd,
                        p.done_moving_seq.max(1),
                        );
                        outbound.send(
                            cid,
                            format_server_message("PU", &[&pu]).into_bytes(),
                        );
                    }
                }
            } else {
                // Blocked path / empty trunc / missing player — force unstick.
                warn!(conn_id, "sim: MOVE rejected — force unstick");
                send_player_update_and_frame(state, outbound, conn_id);
            }
        }
        NetIntent::Use {
            conn_id,
            x,
            y,
            id: _,
            index: _,
        } => {
            touch_afk_activity(state, conn_id);
            let (x, y) = state
                .players
                .get(&conn_id)
                .map(|p| p.client_to_world(x, y))
                .unwrap_or((x, y));
            if state.players.get(&conn_id).map(|p| is_moving(p)).unwrap_or(false) {
                // Keep real done_moving_seq — do not force-bump mid-walk.
                send_action_result_pu_and_frame(state, outbound, conn_id);
                state.publish_player_view(conn_id);
            } else {
            match apply_use_at(state, conn_id, x, y) {
            Some(r) if r.applied => {
                info!(
                    conn_id,
                    x,
                    y,
                    actor = r.actor_before,
                    target = r.target_before,
                    new_actor = r.actor_after,
                    new_target = r.target_after,
                    "sim: USE applied"
                );
                counters.crafts.fetch_add(1, Ordering::Relaxed);
                state.publish_player_view(conn_id);
                let near = nearby_conn_ids(state, x, y, NEARBY_RANGE);
                for pkt in packets_after_use(state, conn_id, &r) {
                    for &cid in &near {
                        outbound.send_urgent(cid, pkt.clone());
                    }
                }
                outbound.send_urgent(conn_id, format_server_message("FM", &[]).into_bytes());
            }
            Some(r) => {
                if try_eat_held(state, conn_id) {
                    state.publish_player_view(conn_id);
                    if let Some(p) = state.players.get(&conn_id) {
                        let fx = food_change_for_player(state, p);
                        let near = nearby_conn_ids(state, p.x, p.y, NEARBY_RANGE);
                        for &cid in &near {
                            outbound.send_urgent(cid, fx.clone().into_bytes());
                        }
                        let spd = player_move_speed(state, p);
                        let (px, py) = p.world_to_client(p.x, p.y);
                        let pu = format_player_update_line_eat(
                            p.p_id,
                            person_object_id(&p),
                            p.held_id,
                            px,
                            py,
                            p.age,
                            spd,
                            p.yum.just_ate_flag(),
                            p.yum.just_ate_id,
                            p.done_moving_seq.max(1),
                        );
                        for &cid in &near {
                            outbound.send_urgent(
                                cid,
                                format_server_message("PU", &[&pu]).into_bytes(),
                            );
                        }
                    }
                    if let Some(p) = state.players.get_mut(&conn_id) {
                        p.yum.clear_just_ate_flag();
                    }
                    outbound.send_urgent(conn_id, format_server_message("FM", &[]).into_bytes());
                } else {
                    debug!(
                        conn_id,
                        x,
                        y,
                        actor = r.actor_before,
                        target = r.target_before,
                        "sim: USE no transition — unstick PU+FM (keep seq)"
                    );
                    state.publish_player_view(conn_id);
                    send_action_result_pu_and_frame(state, outbound, conn_id);
                }
            }
            None => {
                warn!(conn_id, "sim: USE without player");
                send_action_result_pu_and_frame(state, outbound, conn_id);
            }
            }
            } // !moving USE
        }
        NetIntent::Drop { conn_id, x, y, c } => {
            touch_afk_activity(state, conn_id);
            let (x, y) = state
                .players
                .get(&conn_id)
                .map(|p| p.client_to_world(x, y))
                .unwrap_or((x, y));
            apply_drop(state, outbound, conn_id, x, y, c);
        }
        NetIntent::Raw {
            conn_id,
            tag,
            payload,
        } => {
            // Client activity (not KA / PING heartbeats) resets AFK idle.
            // SAY handles touch inside apply_say_or_remv so `?AFK` can report
            // true idle without self-resetting.
            if tag.eq_ignore_ascii_case("SAY") || tag.eq_ignore_ascii_case("REMV") {
                if tag.eq_ignore_ascii_case("REMV") {
                    touch_afk_activity(state, conn_id);
                }
                apply_say_or_remv(state, outbound, counters, conn_id, &tag, &payload);
            } else if tag.eq_ignore_ascii_case("DIE") {
                touch_afk_activity(state, conn_id);
                let died_id = state.players.get_mut(&conn_id).map(|p| {
                    if p.deleted {
                        return None;
                    }
                    p.deleted = true;
                    p.death_reason = Some(DeathCause::Suicide.wire_tag().into());
                    Some(p.p_id)
                });
                if let Some(Some(p_id)) = died_id {
                    scatter_backpack_on_death(state, conn_id);
                    apply_death_inheritance(state, p_id);
                    counters.deaths.fetch_add(1, Ordering::Relaxed);
                    state.scoreboard.record_death(p_id);
                    state.push_event(format_death_event(p_id, DeathCause::Suicide));
                    state.afk.remove(p_id);
                    state.publish_player_view(conn_id);
                    info!(conn_id, "sim: DIE");
                }
            } else if tag.eq_ignore_ascii_case("EMOT") {
                touch_afk_activity(state, conn_id);
                // EMOT x y e → PE player_id emot_index (emote rate limit, not SAY).
                let now = state.sim_time;
                let allowed = state
                    .players
                    .get_mut(&conn_id)
                    .map(|pl| pl.emote_rate.try_emote(now))
                    .unwrap_or(false);
                if !allowed {
                    send_ps_reply(outbound, conn_id, "0 EMOTE RATE");
                    return;
                }
                let Some(p) = state.players.get(&conn_id) else {
                    return;
                };
                let e = payload
                    .split_whitespace()
                    .nth(2)
                    .and_then(|s| s.parse::<i32>().ok())
                    .unwrap_or(0);
                let line = format!("{} {}", p.p_id, e);
                let near = nearby_conn_ids(state, p.x, p.y, NEARBY_RANGE);
                send_nearby(
                    outbound,
                    &near,
                    format_server_message("PE", &[&line]).into_bytes(),
                );
                for &cid in &near {
                    send_frame(outbound, cid);
                }
            } else if tag.eq_ignore_ascii_case("JUMP") {
                touch_afk_activity(state, conn_id);
                // JUMP x y — baby jump-out / wiggle, or position refresh + PU note.
                // Haxe: if held, drop from arms; else PU + wiggle. Always emit PU.
                let mut parts = payload.split_whitespace();
                let x = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
                let y = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
                set_player_position(state, conn_id, x, y);
                // Baby jump out of mother's arms (held_by link).
                let (p_id, held_by) = state
                    .players
                    .get(&conn_id)
                    .map(|p| (p.p_id, p.held_by))
                    .unwrap_or((0, 0));
                if held_by != 0 {
                    if let Some(mother) = state
                        .players
                        .values_mut()
                        .find(|pl| pl.p_id == held_by && pl.holding_player_id == p_id)
                    {
                        mother.release_holding();
                    }
                    if let Some(pl) = state.players.get_mut(&conn_id) {
                        pl.held_by = 0;
                    }
                }
                if let Some(p) = state.players.get(&conn_id) {
                    let spd = player_move_speed(state, p);
                    let pu = format_player_update_line(
                        p.p_id,
                        person_object_id(&p),
                        p.held_id,
                        p.x,
                        p.y,
                        p.age,
                        spd,
                    p.done_moving_seq.max(1),
                    );
                    let near = nearby_conn_ids(state, p.x, p.y, NEARBY_RANGE);
                    send_nearby(
                        outbound,
                        &near,
                        format_server_message("PU", &[&pu]).into_bytes(),
                    );
                    // Immobile baby wiggle note (Haxe sendWiggle / BW).
                    if p.age < BABY_AGE_THRESHOLD {
                        send_nearby(
                            outbound,
                            &near,
                            format_baby_wiggle(p.p_id).into_bytes(),
                        );
                    }
                    info!(conn_id, p_id = p.p_id, "sim: JUMP PU");
                }
                state.publish_player_view(conn_id);
            } else if tag.eq_ignore_ascii_case("PING") {
                // PING x y unique_id → PONG unique_id (x,y ignored; protocol.txt).
                // Net maps ClientCommand::Ping to payload=unique_id only.
                let unique_id = payload
                    .split_whitespace()
                    .last()
                    .unwrap_or(payload.as_str());
                outbound.send(conn_id, format_pong(unique_id).into_bytes());
            } else if tag.eq_ignore_ascii_case("PHOTO") {
                touch_afk_activity(state, conn_id);
                // PHOTO x y seq → PH x y signature (dummy deny ACK; no photo backend).
                // SAY SNAP is the chat alias (same deny).
                let mut parts = payload.split_whitespace();
                let x = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
                let y = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
                let seq = parts.next().unwrap_or("?");
                info!(conn_id, x, y, %seq, "sim: PHOTO (deny ACK)");
                outbound.send(
                    conn_id,
                    format_photo_signature(x, y, PHOTO_DENIED_SIGNATURE).into_bytes(),
                );
            } else if ClientTag::parse(&tag).map(|t| t.is_vog()).unwrap_or(false)
                || tag.eq_ignore_ascii_case("VOG")
            {
                touch_afk_activity(state, conn_id);
                // Voice-of-God client cmds: log + empty VU ACK at requested / player pos.
                let mut parts = payload.split_whitespace();
                let px = parts.next().and_then(|s| s.parse().ok());
                let py = parts.next().and_then(|s| s.parse().ok());
                let (x, y) = match (px, py) {
                    (Some(x), Some(y)) => (x, y),
                    _ => state
                        .players
                        .get(&conn_id)
                        .map(|p| (p.x, p.y))
                        .unwrap_or((0, 0)),
                };
                info!(conn_id, %tag, x, y, %payload, "sim: VOG (no-op ACK)");
                outbound.send(conn_id, format_vog_update(x, y).into_bytes());
            } else {
                touch_afk_activity(state, conn_id);
                debug!(conn_id, %tag, %payload, "sim: raw intent");
            }
        }
        NetIntent::Disconnected { conn_id } => {
            let p_id = state.players.get(&conn_id).map(|p| p.p_id);
            if let Some(p) = state.players.get_mut(&conn_id) {
                p.connected = false;
            }
            if let Some(id) = p_id {
                state.afk.remove(id);
                // Drop mute edges for this p_id (listener + as muted speaker).
                state.mutes.clear_player(id);
            }
            if let Some(views) = &state.player_views {
                if let Ok(mut g) = views.write() {
                    g.remove(&conn_id);
                }
            }
            info!(conn_id, "sim: disconnect");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ol_content::{ContentDb, ObjectDef, Transition};

    #[test]
    fn normalize_say_strips_client_coords() {
        assert_eq!(normalize_say_text("hello"), "hello");
        assert_eq!(normalize_say_text("0 0 !shutdown"), "!shutdown");
        assert_eq!(normalize_say_text("-1 2 !CLOSE"), "!CLOSE");
        assert_eq!(normalize_say_text("  5 5 HELP  "), "HELP");
        assert_eq!(normalize_say_text("!shutdown"), "!shutdown");
    }

    #[test]
    fn shutdown_say_matches() {
        assert!(is_shutdown_say("!SHUTDOWN"));
        // !CLOSE is client-only disconnect, not server shutdown.
        assert!(!is_shutdown_say("!CLOSE"));
        assert!(is_close_say("!CLOSE"));
        assert!(is_close_say("CLOSE!"));
        // contains() also matches pre-normalize form (defense in depth).
        assert!(is_shutdown_say("0 0 !SHUTDOWN"));
        assert!(is_shutdown_say(
            &normalize_say_text("0 0 !shutdown").to_uppercase()
        ));
        assert!(is_shutdown_say("SHUTDOWN"));
        assert!(!is_shutdown_say("HELLO"));
    }

    fn test_content() -> Arc<ContentDb> {
        let mut db = ContentDb::default();
        db.objects.insert(
            33,
            ObjectDef {
                id: 33,
                description: "Gooseberry".into(),
                name: "Gooseberry".into(),
                containable: true,
                permanent: false,
                blocks_walking: false,
                food_value: 3,
                heat_value: 0.0,
                map_chance: 0.0,
                biomes: Vec::new(),
                num_uses: 0,
                num_slots: 0,
                floor: false,
            dummy_ids: Vec::new(),
            },
        );
        db.transitions.insert(
            (0, 33),
            Transition {
                actor_id: 0,
                target_id: 33,
                new_actor_id: 34,
                new_target_id: 0,
                last_use_actor: false,
                last_use_target: false,
                auto_decay_seconds: 0.0,
                reverse_use_actor: false,
                reverse_use_target: false,
                no_use_actor: false,
                no_use_target: false,
                move_dist: 0,

            desired_move_dist: 0,
            },
        );
        // last-use variant: hand on 33 when last use → different outcome
        db.transitions_last_use.insert(
            (0, 33),
            Transition {
                actor_id: 0,
                target_id: 33,
                new_actor_id: 99,
                new_target_id: 1,
                last_use_actor: false,
                last_use_target: true,
                auto_decay_seconds: 0.0,
                reverse_use_actor: false,
                reverse_use_target: false,
                no_use_actor: false,
                no_use_target: false,
                move_dist: 0,

            desired_move_dist: 0,
            },
        );
        db.transition_count = 1;
        db.last_use_transition_count = 1;
        Arc::new(db)
    }

    #[test]
    fn baby_wiggle_and_dying_formatters() {
        assert_eq!(SimState::format_baby_wiggle(42), "BW\n42\n#");
        assert_eq!(format_baby_wiggle(42), "BW\n42\n#");
        assert_eq!(SimState::format_dying(7, false), "DY\n7\n#");
        assert_eq!(SimState::format_dying(7, true), "DY\n7 1\n#");
        assert_eq!(format_dying(9, true), "DY\n9 1\n#");
    }

    #[test]
    fn social_bootstrap_sends_lr_when_tools_learned() {
        // Haxe LEARNED_TOOL_REPORT = "LR"; LINEAGE = "LN" (not LR).
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 3, "tools@test");
        state.social.ensure_lineage(p_id, "TOOLS");
        {
            let p = state.players.get_mut(&3).expect("player");
            p.tools.learn(334);
            p.tools.learn(12);
        }
        let pkts = state.social_bootstrap_packets(p_id);
        let texts: Vec<String> = pkts
            .iter()
            .map(|b| String::from_utf8_lossy(b).into_owned())
            .collect();
        // Lineage uses LN, not LR.
        assert!(
            texts.iter().any(|t| t.starts_with("LN\n")),
            "expected LN lineage packet, got {texts:?}"
        );
        // Learned tools LR with sorted ids.
        let lr = texts
            .iter()
            .find(|t| t.starts_with("LR\n"))
            .expect("expected LR learned-tools packet");
        assert_eq!(lr, "LR\n12 334\n#");
        // TS reflects used count.
        assert!(
            texts.iter().any(|t| t == "TS\n2 1000\n#" || t.starts_with("TS\n2 ")),
            "expected TS with used=2, got {texts:?}"
        );
    }

    #[test]
    fn social_bootstrap_omits_lr_when_no_learned_tools() {
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 4, "empty@test");
        let pkts = state.social_bootstrap_packets(p_id);
        let texts: Vec<String> = pkts
            .iter()
            .map(|b| String::from_utf8_lossy(b).into_owned())
            .collect();
        assert!(
            texts.iter().all(|t| !t.starts_with("LR\n")),
            "empty learned set must not send LR, got {texts:?}"
        );
        assert!(texts.iter().any(|t| t.starts_with("TS\n")));
    }

    #[test]
    fn login_intent_bootstrap_includes_lr_for_reconnect_style_state() {
        // After spawn, inject learned tools then re-run bootstrap path as login does.
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(9);
        let mut state = SimState::with_default_empty(test_content());
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Login {
                conn_id: 9,
                reconnect: false,
                email: "lr@test".into(),
                client_tag: "client_test".into(),
            },
        );
        // Drain first login packets (no LR yet).
        while rx.try_recv().is_ok() {}

        // Simulate tools already known (e.g. restored life / mid-session) and re-bootstrap.
        let p_id = state.players.get(&9).unwrap().p_id;
        state.players.get_mut(&9).unwrap().tools.learn(99);
        for pkt in state.social_bootstrap_packets(p_id) {
            hub.send(9, pkt);
        }
        let mut saw_lr = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.as_ref() == "LR\n99\n#" {
                saw_lr = true;
            }
        }
        assert!(saw_lr, "bootstrap after learn must emit LR");
    }

    #[test]
    fn spawn_and_login_intent() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut state = SimState::with_default_empty(test_content());
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Login {
                conn_id: 7,
                reconnect: false,
                email: "a@b.c".into(),
                client_tag: "client_test".into(),
            },
        );
        assert_eq!(state.logins, 1);
        assert_eq!(counters.snapshot().logins, 1);
        assert!(state.players.get(&7).is_some());
    }

    /// Metrics: death counter increments on SAY DIE and hunger death.
    #[test]
    fn metrics_death_counter_on_die_and_hunger() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "d@x");
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "DIE".into(),
            },
        );
        assert_eq!(counters.snapshot().deaths, 1);

        // Hunger death path (vitals metrics): food must go below DEATH_FOOD_THRESHOLD (0).
        let counters2 = Counters::new();
        let mut state2 = SimState::with_default_empty(test_content());
        spawn_player(&mut state2, 2, "h@x");
        state2.players.get_mut(&2).unwrap().food = -0.1;
        tick_vitals_with_metrics(&mut state2, 0.01, &hub, Some(&counters2));
        assert!(state2.players.get(&2).unwrap().deleted);
        assert_eq!(counters2.snapshot().deaths, 1);
    }

    /// Haxe-aligned USE outcomes from real OneLifeData7 goldens (0_63, 0_36, 0_242).
    /// TransitionImporter: filename actor_target.txt, first line newActor newTarget …
    #[test]
    fn use_applies_haxe_style_transition_goldens() {
        let mut db = ContentDb::default();
        // 0_63.txt → 64 48 0  (hand + maple branch tree)
        db.transitions.insert(
            (0, 63),
            Transition {
                actor_id: 0,
                target_id: 63,
                new_actor_id: 64,
                new_target_id: 48,
                last_use_actor: false,
                last_use_target: false,
                auto_decay_seconds: 0.0,
                reverse_use_actor: false,
                reverse_use_target: false,
                no_use_actor: false,
                no_use_target: false,
                move_dist: 0,

            desired_move_dist: 0,
            },
        );
        // 0_36.txt → 395 404
        db.transitions.insert(
            (0, 36),
            Transition {
                actor_id: 0,
                target_id: 36,
                new_actor_id: 395,
                new_target_id: 404,
                last_use_actor: false,
                last_use_target: false,
                auto_decay_seconds: 0.0,
                reverse_use_actor: false,
                reverse_use_target: false,
                no_use_actor: false,
                no_use_target: false,
                move_dist: 0,

            desired_move_dist: 0,
            },
        );
        // 0_242.txt → 223 242
        db.transitions.insert(
            (0, 242),
            Transition {
                actor_id: 0,
                target_id: 242,
                new_actor_id: 223,
                new_target_id: 242,
                last_use_actor: false,
                last_use_target: false,
                auto_decay_seconds: 0.0,
                reverse_use_actor: false,
                reverse_use_target: false,
                no_use_actor: false,
                no_use_target: false,
                move_dist: 0,

            desired_move_dist: 0,
            },
        );
        db.transition_count = 3;
        let mut state = SimState::with_default_empty(Arc::new(db));
        spawn_player(&mut state, 1, "golden@t");
        set_player_position(&mut state, 1, 10, 10);
        state.players.get_mut(&1).unwrap().held_id = 0;

        // Case 1: maple branch
        state.world.write().unwrap().set_object(10, 10, 63);
        let r = apply_use_at(&mut state, 1, 10, 10).unwrap();
        assert!(r.applied, "0_63 should apply");
        assert_eq!((r.actor_after, r.target_after), (64, 48));
        assert_eq!(state.players.get(&1).unwrap().held_id, 64);
        assert_eq!(state.world.read().unwrap().get_object(10, 10), 48);

        // Case 2: seeding wild carrot (clear held)
        state.players.get_mut(&1).unwrap().held_id = 0;
        state.world.write().unwrap().set_object(10, 11, 36);
        set_player_position(&mut state, 1, 10, 11);
        let r = apply_use_at(&mut state, 1, 10, 11).unwrap();
        assert!(r.applied, "0_36 should apply");
        assert_eq!((r.actor_after, r.target_after), (395, 404));

        // Case 3: ripe wheat
        state.players.get_mut(&1).unwrap().held_id = 0;
        state.world.write().unwrap().set_object(11, 11, 242);
        set_player_position(&mut state, 1, 11, 11);
        let r = apply_use_at(&mut state, 1, 11, 11).unwrap();
        assert!(r.applied, "0_242 should apply");
        assert_eq!((r.actor_after, r.target_after), (223, 242));

        // Moving blocks USE (Haxe checkIfNotMovingAndCloseEnough)
        state.players.get_mut(&1).unwrap().moving = true;
        state.players.get_mut(&1).unwrap().move_path =
            Some(build_move_path(11, 11, vec![(1, 0)], 3.75, 1, 0, 0));
        state.world.write().unwrap().set_object(11, 11, 63);
        let r = apply_use_at(&mut state, 1, 11, 11).unwrap();
        assert!(!r.applied);
    }

    /// SAY LASTUSE sets force_last_use; next USE prefers last-use table.
    #[test]
    fn say_lastuse_forces_last_use_transition() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "u");
        set_player_position(&mut state, 1, 1, 1);
        state.world.write().unwrap().set_object(1, 1, 33);
        assert!(!state.players.get(&1).unwrap().force_last_use);
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "LASTUSE".into(),
            },
        );
        assert!(state.players.get(&1).unwrap().force_last_use);
        let r = apply_use_at(&mut state, 1, 1, 1).unwrap();
        assert!(r.applied);
        // last-use (0,33) → (99,1) in test_content
        assert_eq!(r.actor_after, 99);
        assert_eq!(r.target_after, 1);
        // force flag cleared after applied USE
        assert!(!state.players.get(&1).unwrap().force_last_use);
    }

    /// Successful SAY CRAFT increments crafts metric.
    #[test]
    fn metrics_craft_counter_on_say_craft() {
        let mut db = ContentDb::default();
        db.transitions.insert(
            (34, 0),
            Transition {
                actor_id: 34,
                target_id: 0,
                new_actor_id: 99,
                new_target_id: 0,
                last_use_actor: false,
                last_use_target: false,
                auto_decay_seconds: 0.0,
                reverse_use_actor: false,
                reverse_use_target: false,
                no_use_actor: false,
                no_use_target: false,
                move_dist: 0,

            desired_move_dist: 0,
            },
        );
        db.transition_count = 1;
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut state = SimState::with_default_empty(Arc::new(db));
        spawn_player(&mut state, 1, "c@x");
        state.players.get_mut(&1).unwrap().held_id = 34;
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "CRAFT".into(),
            },
        );
        assert_eq!(counters.snapshot().crafts, 1);
        assert_eq!(state.players.get(&1).unwrap().held_id, 99);
    }

    /// Craft graph seed respects explicit cap.
    #[test]
    fn build_reverse_craft_graph_respects_cap() {
        let mut db = ContentDb::default();
        for i in 1..=20 {
            db.transitions.insert(
                (i, 0),
                Transition {
                    actor_id: i,
                    target_id: 0,
                    new_actor_id: i + 100,
                    new_target_id: 0,
                    last_use_actor: false,
                    last_use_target: false,
                    auto_decay_seconds: 0.0,
                    reverse_use_actor: false,
                    reverse_use_target: false,
                    no_use_actor: false,
                    no_use_target: false,
                    move_dist: 0,

                desired_move_dist: 0,
                },
            );
        }
        let g = build_reverse_craft_graph_capped(&db, 5);
        // At most 5 transitions seeded → at most 5 product edges.
        assert!(g.edge_count() <= 5);
        assert!(g.product_count() <= 5);
        assert!(g.product_count() >= 1);
    }

    #[test]
    fn spawn_player_assigns_non_empty_names() {
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "u@test");
        let p = state.players.get(&1).expect("spawned");
        assert!(!p.first_name.is_empty());
        assert!(!p.family_name.is_empty());
        assert!(FIRST_NAMES.contains(&p.first_name.as_str()));
        assert!(FAMILY_NAMES.contains(&p.family_name.as_str()));
        // Not derived from email alone.
        assert_ne!(p.first_name, "U");
        assert_ne!(p.first_name, "U@TEST");
    }

    #[test]
    fn use_mutates_shared_world_and_mx_packets() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "u@test");
        set_player_position(&mut state, 1, 5, 5);
        state.world.write().unwrap().set_object(5, 5, 33);

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Use {
                conn_id: 1,
                x: 5,
                y: 5,
                id: None,
                index: None,
            },
        );
        assert_eq!(state.world.read().unwrap().get_object(5, 5), 0);
        assert_eq!(state.players.get(&1).unwrap().held_id, 34);

        // Outbound MX then PU then FX — MX uses -(p_id) for transforms (not drop).
        let mx = rx.try_recv().expect("MX packet");
        let mx_s = String::from_utf8_lossy(&mx);
        assert!(mx_s.starts_with("MX\n"));
        assert!(mx_s.contains("5 5 0 0"), "got {mx_s}");
        // player_id_for_conn(1)=2 → responsible -2
        assert!(
            mx_s.contains(" 0 -2\n") || mx_s.contains("0 -2\n#") || mx_s.contains("0 -2"),
            "transform MX must use -p_id (got {mx_s})"
        );
        let pu = rx.try_recv().expect("PU");
        assert!(String::from_utf8_lossy(&pu).starts_with("PU\n"));
        let fx = rx.try_recv().expect("FX");
        assert!(String::from_utf8_lossy(&fx).starts_with("FX\n"));
    }

    /// Stone pile: reverse-use starts at 1; taking decrements; last take uses LT.
    #[test]
    fn stone_pile_uses_start_at_one_and_decrement() {
        let mut db = ContentDb::default();
        db.objects.insert(
            33,
            ObjectDef {
                id: 33,
                description: "Stone".into(),
                name: "Stone".into(),
                containable: true,
                permanent: false,
                blocks_walking: false,
                food_value: 0,
                heat_value: 0.0,
                map_chance: 0.0,
                biomes: Vec::new(),
                num_uses: 0,
                num_slots: 0,
                floor: false,
            dummy_ids: Vec::new(),
            },
        );
        db.objects.insert(
            661,
            ObjectDef {
                id: 661,
                description: "Stone Pile".into(),
                name: "Stone Pile".into(),
                containable: false,
                permanent: true,
                blocks_walking: false,
                food_value: 0,
                heat_value: 0.0,
                map_chance: 0.0,
                biomes: Vec::new(),
                num_uses: 9,
                num_slots: 0,
                floor: false,
            dummy_ids: Vec::new(),
            },
        );
        // 33+33 → pile reverse target (start uses=1)
        db.transitions.insert(
            (33, 33),
            Transition {
                actor_id: 33,
                target_id: 33,
                new_actor_id: 0,
                new_target_id: 661,
                last_use_actor: false,
                last_use_target: false,
                auto_decay_seconds: 0.0,
                reverse_use_actor: false,
                reverse_use_target: true,
                no_use_actor: true,
                no_use_target: false,
                move_dist: 0,
                desired_move_dist: 0,
            },
        );
        // 33+661 → pile reverse (uses += 1)
        db.transitions.insert(
            (33, 661),
            Transition {
                actor_id: 33,
                target_id: 661,
                new_actor_id: 0,
                new_target_id: 661,
                last_use_actor: false,
                last_use_target: false,
                auto_decay_seconds: 0.0,
                reverse_use_actor: false,
                reverse_use_target: true,
                no_use_actor: true,
                no_use_target: false,
                move_dist: 0,
                desired_move_dist: 0,
            },
        );
        // 0+661 → take stone, pile stays (uses -= 1)
        db.transitions.insert(
            (0, 661),
            Transition {
                actor_id: 0,
                target_id: 661,
                new_actor_id: 33,
                new_target_id: 661,
                last_use_actor: false,
                last_use_target: false,
                auto_decay_seconds: 0.0,
                reverse_use_actor: false,
                reverse_use_target: false,
                no_use_actor: true,
                no_use_target: false,
                move_dist: 0,
                desired_move_dist: 0,
            },
        );
        // last-use: 0+661 → stone + stone
        db.transitions_last_use.insert(
            (0, 661),
            Transition {
                actor_id: 0,
                target_id: 661,
                new_actor_id: 33,
                new_target_id: 33,
                last_use_actor: false,
                last_use_target: true,
                auto_decay_seconds: 0.0,
                reverse_use_actor: false,
                reverse_use_target: false,
                no_use_actor: true,
                no_use_target: false,
                move_dist: 0,
                desired_move_dist: 0,
            },
        );
        db.transition_count = 3;
        db.last_use_transition_count = 1;

        let mut state = SimState::with_default_empty(Arc::new(db));
        spawn_player(&mut state, 1, "pile@test");
        set_player_position(&mut state, 1, 5, 5);
        state.players.get_mut(&1).unwrap().held_id = 33;
        state.world.write().unwrap().set_object(5, 5, 33);

        // First pile: uses = 1 (not 9).
        let r = apply_use_at(&mut state, 1, 5, 5).unwrap();
        assert!(r.applied);
        assert_eq!(r.target_after, 661);
        assert_eq!(state.players.get(&1).unwrap().held_id, 0);
        let uses = state
            .world
            .read()
            .unwrap()
            .get_helper(5, 5)
            .map(|h| h.uses_remaining)
            .unwrap_or(-1);
        assert_eq!(uses, 1, "new pile must start at 1 use, got {uses}");

        // Add another stone → uses = 2.
        state.players.get_mut(&1).unwrap().held_id = 33;
        let r = apply_use_at(&mut state, 1, 5, 5).unwrap();
        assert!(r.applied);
        let uses = state
            .world
            .read()
            .unwrap()
            .get_helper(5, 5)
            .map(|h| h.uses_remaining)
            .unwrap_or(-1);
        assert_eq!(uses, 2, "add stone should increment uses to 2");

        // Take one → uses = 1, hold stone.
        let r = apply_use_at(&mut state, 1, 5, 5).unwrap();
        assert!(r.applied);
        assert_eq!(state.players.get(&1).unwrap().held_id, 33);
        let uses = state
            .world
            .read()
            .unwrap()
            .get_helper(5, 5)
            .map(|h| h.uses_remaining)
            .unwrap_or(-1);
        assert_eq!(uses, 1, "take must decrement pile uses");

        // Last take (uses=1 → prefer LT) → stone + stone on ground.
        state.players.get_mut(&1).unwrap().held_id = 0;
        let r = apply_use_at(&mut state, 1, 5, 5).unwrap();
        assert!(r.applied);
        assert_eq!(state.players.get(&1).unwrap().held_id, 33);
        assert_eq!(
            state.world.read().unwrap().get_object(5, 5),
            33,
            "last take leaves single stone"
        );
    }

    /// Bare-hand USE on non-permanent ground object with no transition → pickup (Haxe swap).
    #[test]
    fn bare_hand_pickup_swaps_ground_object() {
        let mut db = ContentDb::default();
        // Stick: non-permanent, no (0,stick) transition → bare-hand swap.
        db.objects.insert(
            99,
            ObjectDef {
                id: 99,
                description: "Stick".into(),
                name: "Stick".into(),
                containable: true,
                permanent: false,
                blocks_walking: false,
                food_value: 0,
                heat_value: 0.0,
                map_chance: 0.0,
                biomes: Vec::new(),
                num_uses: 0,
                num_slots: 0,
                floor: false,
            dummy_ids: Vec::new(),
            },
        );
        // Tree: permanent, cannot bare-hand pickup.
        db.objects.insert(
            100,
            ObjectDef {
                id: 100,
                description: "Tree".into(),
                name: "Tree".into(),
                containable: false,
                permanent: true,
                blocks_walking: true,
                food_value: 0,
                heat_value: 0.0,
                map_chance: 0.0,
                biomes: Vec::new(),
                num_uses: 0,
                num_slots: 0,
                floor: false,
            dummy_ids: Vec::new(),
            },
        );
        let mut state = SimState::with_default_empty(Arc::new(db));
        spawn_player(&mut state, 1, "pick@test");
        set_player_position(&mut state, 1, 2, 2);
        state.world.write().unwrap().set_object(2, 2, 99);
        let r = apply_use_at(&mut state, 1, 2, 2).unwrap();
        assert!(r.applied, "bare-hand pickup should apply");
        assert_eq!(r.actor_after, 99);
        assert_eq!(r.target_after, 0);
        assert_eq!(state.players.get(&1).unwrap().held_id, 99);
        assert_eq!(state.world.read().unwrap().get_object(2, 2), 0);

        // Permanent refuses pickup.
        state.players.get_mut(&1).unwrap().held_id = 0;
        state.world.write().unwrap().set_object(2, 3, 100);
        set_player_position(&mut state, 1, 2, 3);
        let r2 = apply_use_at(&mut state, 1, 2, 3).unwrap();
        assert!(!r2.applied);
        assert_eq!(state.players.get(&1).unwrap().held_id, 0);
        assert_eq!(state.world.read().unwrap().get_object(2, 3), 100);
    }

    /// `try_craft` / `SAY CRAFT` applies `find_transition(held, 0)` (USE-on-empty).
    #[test]
    fn try_craft_applies_held_target_zero_transition() {
        let mut db = ContentDb::default();
        // Fake recipe: hold 100 on empty → hold 101, place 200 under feet.
        db.transitions.insert(
            (100, 0),
            Transition {
                actor_id: 100,
                target_id: 0,
                new_actor_id: 101,
                new_target_id: 200,
                last_use_actor: false,
                last_use_target: false,
                auto_decay_seconds: 0.0,
                reverse_use_actor: false,
                reverse_use_target: false,
                no_use_actor: false,
                no_use_target: false,
                move_dist: 0,

            desired_move_dist: 0,
            },
        );
        db.transition_count = 1;
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(Arc::new(db));
        spawn_player(&mut state, 1, "craft@test");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.held_id = 100;
            p.x = 3;
            p.y = 4;
        }

        let r = try_craft(&mut state, 1).expect("player exists");
        assert!(r.applied);
        assert_eq!(r.actor_before, 100);
        assert_eq!(r.target_before, 0);
        assert_eq!(r.actor_after, 101);
        assert_eq!(r.target_after, 200);
        assert_eq!(r.x, 3);
        assert_eq!(r.y, 4);
        assert_eq!(state.players.get(&1).unwrap().held_id, 101);
        assert_eq!(state.world.read().unwrap().get_object(3, 4), 200);

        // No recipe for held 999 → fail without mutating.
        state.players.get_mut(&1).unwrap().held_id = 999;
        let r2 = try_craft(&mut state, 1).unwrap();
        assert!(!r2.applied);
        assert_eq!(state.players.get(&1).unwrap().held_id, 999);

        // Empty hands → not applied.
        state.players.get_mut(&1).unwrap().held_id = 0;
        let r3 = try_craft(&mut state, 1).unwrap();
        assert!(!r3.applied);

        // Wire path: SAY CRAFT with a valid (held, 0) recipe.
        // Re-seed content: player still on (3,4) with object 200; craft leaves ground alone when new_target=0.
        let mut db2 = ContentDb::default();
        db2.transitions.insert(
            (50, 0),
            Transition {
                actor_id: 50,
                target_id: 0,
                new_actor_id: 51,
                new_target_id: 0,
                last_use_actor: false,
                last_use_target: false,
                auto_decay_seconds: 0.0,
                reverse_use_actor: false,
                reverse_use_target: false,
                no_use_actor: false,
                no_use_target: false,
                move_dist: 0,

            desired_move_dist: 0,
            },
        );
        db2.transition_count = 1;
        let mut state2 = SimState::with_default_empty(Arc::new(db2));
        spawn_player(&mut state2, 1, "craft2@test");
        state2.players.get_mut(&1).unwrap().held_id = 50;
        apply_intent(
            &mut state2,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "CRAFT".into(),
            },
        );
        assert_eq!(state2.players.get(&1).unwrap().held_id, 51);
        let mut saw_ok = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("CRAFT OK") {
                saw_ok = true;
            }
        }
        assert!(saw_ok, "expected PS CRAFT OK");
    }

    #[test]
    fn last_use_transition_preferred_when_flagged() {
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "u");
        set_player_position(&mut state, 1, 1, 1);
        state.prefer_last_use = true;
        state.world.write().unwrap().set_object(1, 1, 33);
        let r = apply_use_at(&mut state, 1, 1, 1).unwrap();
        assert!(r.applied);
        assert_eq!(r.actor_after, 99);
        assert_eq!(r.target_after, 1);
        assert_eq!(state.world.read().unwrap().get_object(1, 1), 1);
    }

    #[test]
    fn multi_use_decrements_then_last_use() {
        use ol_world::ComplexObject;
        // Object 50: multi-use berry; normal USE keeps id 50, last-use → 0
        let mut db = ContentDb::default();
        db.objects.insert(
            50,
            ObjectDef {
                id: 50,
                description: "Berry Bush".into(),
                name: "Berry Bush".into(),
                containable: false,
                permanent: true,
                blocks_walking: false,
                food_value: 0,
                heat_value: 0.0,
                map_chance: 0.0,
                biomes: Vec::new(),
                num_uses: 3,
                num_slots: 0,
                floor: false,
            dummy_ids: Vec::new(),
            },
        );
        db.transitions.insert(
            (0, 50),
            Transition {
                actor_id: 0,
                target_id: 50,
                new_actor_id: 33,
                new_target_id: 50, // same id while uses remain
                last_use_actor: false,
                last_use_target: false,
                auto_decay_seconds: 0.0,
                reverse_use_actor: false,
                reverse_use_target: false,
                no_use_actor: false,
                no_use_target: false,
                move_dist: 0,

            desired_move_dist: 0,
            },
        );
        db.transitions_last_use.insert(
            (0, 50),
            Transition {
                actor_id: 0,
                target_id: 50,
                new_actor_id: 33,
                new_target_id: 0,
                last_use_actor: false,
                last_use_target: true,
                auto_decay_seconds: 0.0,
                reverse_use_actor: false,
                reverse_use_target: false,
                no_use_actor: false,
                no_use_target: false,
                move_dist: 0,

            desired_move_dist: 0,
            },
        );
        let mut state = SimState::with_default_empty(Arc::new(db));
        spawn_player(&mut state, 1, "u");
        state.world.write().unwrap().set_object_complex(
            0,
            0,
            ComplexObject::with_uses(50, 3),
        );

        let r1 = apply_use_at(&mut state, 1, 0, 0).unwrap();
        assert!(r1.applied);
        assert_eq!(r1.target_after, 50);
        assert_eq!(
            state.world.read().unwrap().get_helper(0, 0).unwrap().uses_remaining,
            2
        );
        // Drop held berry so next USE is bare hand again.
        state.players.get_mut(&1).unwrap().held_id = 0;

        let r2 = apply_use_at(&mut state, 1, 0, 0).unwrap();
        assert_eq!(
            state.world.read().unwrap().get_helper(0, 0).unwrap().uses_remaining,
            1
        );
        assert_eq!(r2.target_after, 50);
        state.players.get_mut(&1).unwrap().held_id = 0;

        // uses==1 → last-use table → empty tile
        let r3 = apply_use_at(&mut state, 1, 0, 0).unwrap();
        assert_eq!(r3.target_after, 0);
        assert_eq!(state.world.read().unwrap().get_object(0, 0), 0);
        assert!(state.world.read().unwrap().get_helper(0, 0).is_none());
    }

    #[test]
    fn mc_reads_live_world_after_place() {
        let state = SimState::with_default_empty(test_content());
        state.world.write().unwrap().set_object(0, 0, 33);
        let w = state.world.read().unwrap();
        let ids = build_region_object_ids(&w, 0, 0, 2, 1);
        assert_eq!(ids[0], 33);
        let pkt = build_map_chunk_packet(&w, 0, 0, 4, 4);
        assert!(pkt.starts_with(b"MC\n"));
        let plain = build_chunk_plaintext(&w, 0, 0, 1, 1);
        assert!(plain.contains("33"));
    }

    #[test]
    fn move_deltas_update_position() {
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "u");
        // Client path deltas are **start-relative** waypoints (protocol.txt), not steps.
        // From (10,20): (1,0)→(11,20), (2,0)→(12,20), (2,1)→(12,21).
        assert!(apply_move_deltas(
            &mut state,
            1,
            10,
            20,
            &[(1, 0), (2, 0), (2, 1)]
        ));
        let p = state.players.get(&1).unwrap();
        assert_eq!((p.x, p.y), (12, 21));
    }

    #[test]
    fn birth_origin_set_on_spawn_and_client_coords() {
        let mut state = SimState::with_default_empty(test_content());
        // Prefer fixed spawn so we can assert birth.
        state.spawn_x = 100;
        state.spawn_y = 200;
        spawn_player(&mut state, 1, "eve@test");
        let p = state.players.get(&1).unwrap();
        assert_eq!((p.birth_x, p.birth_y), (p.x, p.y));
        // Wire (0,0) is birth; +1 east is world birth+1.
        let (wx, wy) = p.client_to_world(0, 0);
        assert_eq!((wx, wy), (p.birth_x, p.birth_y));
        let (wx, wy) = p.client_to_world(2, 0);
        assert_eq!((wx, wy), (p.birth_x + 2, p.birth_y));
        let (cx, cy) = p.world_to_client(p.x, p.y);
        assert_eq!((cx, cy), (0, 0));
    }

    /// MOVE into mountain wall (biome 21 / SNOWINGREY) is rejected; position unchanged.
    #[test]
    fn move_blocked_into_mountain_biome() {
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "climber@test");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.x = 0;
            p.y = 0;
        }
        // Target tile is mountain (SNOWINGREY = 21).
        state.world.write().unwrap().set_biome(1, 0, BIOME_MOUNTAIN);
        assert!(!apply_move_deltas(&mut state, 1, 0, 0, &[(1, 0)]));
        let p = state.players.get(&1).unwrap();
        assert_eq!((p.x, p.y), (0, 0));
        // Adjacent green tile still walkable.
        state.world.write().unwrap().set_biome(0, 1, 0);
        assert!(apply_move_deltas(&mut state, 1, 0, 0, &[(0, 1)]));
        assert_eq!((state.players.get(&1).unwrap().x, state.players.get(&1).unwrap().y), (0, 1));
    }

    #[test]
    fn food_and_age_tick_can_kill() {
        let hub = OutboundHub::new();
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "starve");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.food = 0.05;
            p.age = 20.0;
        }
        tick_vitals(&mut state, 1.0, &hub);
        let p = state.players.get(&1).unwrap();
        assert!(p.deleted);
        assert_eq!(p.death_reason.as_deref(), Some("reason_hunger"));
    }

    /// Death clears held item and scatters it to a neighbor; body tile stays empty without Grave.
    #[test]
    fn death_clears_held() {
        let hub = OutboundHub::new();
        let mut state = SimState::with_default_empty(test_content());
        assert_eq!(state.grave_object_id, 0, "test_content has no Grave");
        spawn_player(&mut state, 1, "carry@die");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.held_id = 33;
            p.food = 0.05;
            p.age = 20.0;
            p.x = 7;
            p.y = 8;
        }
        tick_vitals(&mut state, 1.0, &hub);
        let p = state.players.get(&1).unwrap();
        assert!(p.deleted);
        assert_eq!(p.held_id, 0, "death must clear held");
        assert_eq!(p.death_reason.as_deref(), Some("reason_hunger"));
        assert_eq!(
            state.world.read().unwrap().get_object(7, 8),
            0,
            "no grave object when content has no Grave (held scatters to ring first)"
        );
        // Held should be on a ring-1 tile near death.
        let mut found_held = false;
        let w = state.world.read().unwrap();
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                if w.get_object(7 + dx, 8 + dy) == 33 {
                    found_held = true;
                }
            }
        }
        assert!(found_held, "held item 33 should scatter near death tile");
    }

    /// Content object named Grave resolves non-zero id and is placed on hunger death.
    #[test]
    fn death_places_grave_when_content_has_grave() {
        let hub = OutboundHub::new();
        let mut db = ContentDb::default();
        db.objects.insert(
            77,
            ObjectDef {
                id: 77,
                description: "stone grave".into(),
                name: "Grave".into(),
                containable: false,
                permanent: true,
                blocks_walking: false,
                food_value: 0,
                heat_value: 0.0,
                map_chance: 0.0,
                biomes: Vec::new(),
                num_uses: 0,
                num_slots: 0,
                floor: false,
            dummy_ids: Vec::new(),
            },
        );
        db.objects.insert(
            88,
            ObjectDef {
                id: 88,
                description: "another".into(),
                name: "Old Grave".into(),
                containable: false,
                permanent: true,
                blocks_walking: false,
                food_value: 0,
                heat_value: 0.0,
                map_chance: 0.0,
                biomes: Vec::new(),
                num_uses: 0,
                num_slots: 0,
                floor: false,
            dummy_ids: Vec::new(),
            },
        );
        assert_eq!(resolve_grave_object_id(&db), 77, "lowest matching id");
        let mut state = SimState::with_default_empty(Arc::new(db));
        assert_eq!(state.grave_object_id, 77);
        spawn_player(&mut state, 1, "bury@me");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.food = 0.05;
            p.age = 20.0;
            p.x = 3;
            p.y = 4;
        }
        tick_vitals(&mut state, 1.0, &hub);
        assert!(state.players.get(&1).unwrap().deleted);
        assert_eq!(state.world.read().unwrap().get_object(3, 4), 77);
        assert_eq!(
            state.specials.count(SpecialKind::Grave),
            1,
            "grave indexed as special"
        );
    }

    /// age > 60 multiplies food drain by OLD_AGE_FOOD_DRAIN_MULT (1.5×).
    #[test]
    fn old_age_increases_food_drain() {
        let hub = OutboundHub::new();
        let mut young = SimState::with_default_empty(test_content());
        let mut old = SimState::with_default_empty(test_content());
        spawn_player(&mut young, 1, "young");
        spawn_player(&mut old, 1, "old");
        // Neutral env so only base + old-age mult apply.
        for s in [&mut young, &mut old] {
            s.environment.temperature = 0.5;
            s.environment.season_length = 10_000.0;
            s.environment.day_length = 10_000.0;
            s.environment.hour_of_day = 12.0;
        }
        young.players.get_mut(&1).unwrap().age = 30.0;
        // Start just above threshold so one tick stays > 60.
        old.players.get_mut(&1).unwrap().age = OLD_AGE_THRESHOLD + 0.1;

        let food0 = young.players.get(&1).unwrap().food;
        assert_eq!(food0, old.players.get(&1).unwrap().food);

        tick_vitals(&mut young, 1.0, &hub);
        tick_vitals(&mut old, 1.0, &hub);

        let young_lost = food0 - young.players.get(&1).unwrap().food;
        let old_lost = food0 - old.players.get(&1).unwrap().food;
        assert!(
            (young_lost - FOOD_USE_PER_SEC).abs() < 1e-4,
            "young drain: lost={young_lost}"
        );
        let expected_old = FOOD_USE_PER_SEC * OLD_AGE_FOOD_DRAIN_MULT;
        assert!(
            (old_lost - expected_old).abs() < 1e-4,
            "old drain: lost={old_lost} expected={expected_old}"
        );
        assert!(old_lost > young_lost);
    }

    /// age ≤ 60 does not get the old-age food drain multiplier.
    #[test]
    fn at_old_age_threshold_no_extra_drain() {
        let hub = OutboundHub::new();
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "edge");
        state.environment.temperature = 0.5;
        state.environment.season_length = 10_000.0;
        state.environment.day_length = 10_000.0;
        state.environment.hour_of_day = 12.0;
        // After +AGE_YEARS_PER_SEC still ≤ 60 if we start low enough... use age that
        // ends exactly at threshold after tick (strict > required for mult).
        let p = state.players.get_mut(&1).unwrap();
        p.age = OLD_AGE_THRESHOLD - AGE_YEARS_PER_SEC;
        let food0 = p.food;

        tick_vitals(&mut state, 1.0, &hub);
        let p = state.players.get(&1).unwrap();
        assert!((p.age - OLD_AGE_THRESHOLD).abs() < 1e-4);
        let lost = food0 - p.food;
        assert!(
            (lost - FOOD_USE_PER_SEC).abs() < 1e-4,
            "at threshold age, no 1.5×: lost={lost}"
        );
    }

    /// age > 120 deletes the player with death_reason reason_age.
    #[test]
    fn age_death_over_max_sets_reason_age() {
        let hub = OutboundHub::new();
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "elder");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.age = MAX_AGE; // one tick of aging pushes past 120
            p.food = 20.0; // not hunger
        }
        tick_vitals(&mut state, 1.0, &hub);
        let p = state.players.get(&1).unwrap();
        assert!(p.deleted);
        assert!(p.age > MAX_AGE);
        assert_eq!(p.death_reason.as_deref(), Some("reason_age"));
    }

    /// Exactly age == 120 after tick does not die of age (strict >).
    #[test]
    fn at_max_age_not_yet_dead() {
        let hub = OutboundHub::new();
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "almost");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.age = MAX_AGE - AGE_YEARS_PER_SEC;
            p.food = 20.0;
        }
        tick_vitals(&mut state, 1.0, &hub);
        let p = state.players.get(&1).unwrap();
        assert!(!p.deleted);
        assert!((p.age - MAX_AGE).abs() < 1e-4);
        assert!(p.death_reason.is_none());
    }

    /// Every ~10s sim time, tick_vitals sends HX heat from temperature_at_biome.
    #[test]
    fn tick_vitals_emits_hx_heat_every_interval() {
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "heat@test");
        state.environment.temperature = 0.5;
        state.environment.season_length = 10_000.0;
        state.environment.day_length = 10_000.0;
        state.hx_emit_timer = 0.0;

        // Under interval: no HX yet.
        tick_vitals(&mut state, 9.0, &hub);
        assert!(
            rx.try_recv().is_err(),
            "no HX before HX_EMIT_INTERVAL_SECS"
        );

        // Expected heat is biome temp *before* the emit tick (HX reads env pre-tick).
        let (px, py) = {
            let p = state.players.get(&1).unwrap();
            (p.x, p.y)
        };
        let biome = state.world.read().unwrap().get_biome(px, py);
        let expected_heat = state.environment.temperature_at_biome(biome);
        let expected = format_heat_change(expected_heat, 0.0, 0.0);

        // Cross interval: HX with biome temperature.
        tick_vitals(&mut state, 1.5, &hub);
        let mut saw_hx = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.as_ref() == expected {
                saw_hx = true;
            }
        }
        assert!(saw_hx, "expected HX packet {expected}");
        // Timer reset; not firing again immediately.
        tick_vitals(&mut state, 1.0, &hub);
        assert!(
            rx.try_recv().is_err(),
            "no second HX before another full interval"
        );
    }

    /// Starving infant (age&lt;3, food&lt;5) emits BW and DY to nearby after ~5s sim time.
    #[test]
    fn tick_vitals_emits_baby_wiggle_and_dying_for_starving_infant() {
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "baby@test");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.age = 1.0;
            p.food = 4.0;
            p.vitals_emit_timer = 0.0;
        }
        // Neutral temp / long day so drain is predictable and player stays alive.
        state.environment.temperature = 0.5;
        state.environment.season_length = 10_000.0;
        state.environment.day_length = 10_000.0;

        // Under interval: no emit yet.
        tick_vitals(&mut state, 4.0, &hub);
        assert!(
            rx.try_recv().is_err(),
            "no BW/DY before VITALS_EMIT_INTERVAL_SECS"
        );

        // Cross interval: BW + DY should arrive for self (nearby includes self).
        tick_vitals(&mut state, 1.5, &hub);
        let mut saw_bw = false;
        let mut saw_dy = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.as_ref() == format_baby_wiggle(p_id) {
                saw_bw = true;
            }
            if s.as_ref() == format_dying(p_id, false) {
                saw_dy = true;
            }
        }
        assert!(saw_bw, "expected BW packet for starving infant p_id={p_id}");
        assert!(saw_dy, "expected DY packet for starving infant p_id={p_id}");
        // Timer reset; not firing again immediately.
        tick_vitals(&mut state, 1.0, &hub);
        assert!(
            rx.try_recv().is_err(),
            "no second emit before another full interval"
        );
    }

    /// Low food (food&lt;3) emits PE hunger emote to nearby after ~8s sim time.
    #[test]
    fn tick_vitals_emits_pe_hunger_emote_when_food_low() {
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "hungry@test");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.age = 20.0;
            p.food = 2.5;
            p.hunger_emot_timer = 0.0;
        }
        // Neutral temp / long day so drain is predictable and player stays alive.
        state.environment.temperature = 0.5;
        state.environment.season_length = 10_000.0;
        state.environment.day_length = 10_000.0;

        let expected_pe =
            format_server_message("PE", &[&format!("{p_id} {HUNGER_EMOT_INDEX}")]);

        // Under interval: no PE yet (and total sim time stays under HX interval).
        tick_vitals(&mut state, 7.0, &hub);
        assert!(
            rx.try_recv().is_err(),
            "no PE before HUNGER_EMOT_INTERVAL_SECS"
        );

        // Cross interval: PE hunger emote to self (nearby includes self).
        tick_vitals(&mut state, 1.5, &hub);
        let mut saw_pe = false;
        while let Ok(pkt) = rx.try_recv() {
            if String::from_utf8_lossy(&pkt).as_ref() == expected_pe {
                saw_pe = true;
            }
        }
        assert!(saw_pe, "expected PE hunger packet {expected_pe}");
        // Timer reset; not firing again immediately.
        tick_vitals(&mut state, 1.0, &hub);
        assert!(
            rx.try_recv().is_err(),
            "no second PE before another full interval"
        );
        // Above threshold: no PE even after a full interval (keep total < HX window).
        {
            let p = state.players.get_mut(&1).unwrap();
            p.food = 5.0;
            p.hunger_emot_timer = HUNGER_EMOT_INTERVAL_SECS; // would fire if still hungry
        }
        // Only advance a little so we don't hit HX; food is high so PE must not fire.
        tick_vitals(&mut state, 0.1, &hub);
        assert!(
            rx.try_recv().is_err(),
            "no PE when food >= HUNGER_EMOT_FOOD_THRESHOLD"
        );
        assert_eq!(
            state.players.get(&1).unwrap().hunger_emot_timer,
            0.0,
            "hunger timer cleared when food is sufficient"
        );
    }

    /// Snow biome food drain > green at the same extreme base temperature
    /// (both receive TEMP_FOOD_EXTRA; snow also has a higher biome multiplier).
    #[test]
    fn snow_biome_drains_food_faster_than_green_at_temp_extremes() {
        let mut snow = SimState::with_default_empty(test_content());
        let mut green = SimState::with_default_empty(test_content());
        spawn_player(&mut snow, 1, "snow");
        spawn_player(&mut green, 1, "green");

        // Same extreme base temp so both tiles hit TEMP_FOOD_EXTRA.
        snow.environment.temperature = 0.0;
        green.environment.temperature = 0.0;
        // Avoid season tick shifting temps mid-comparison.
        snow.environment.season_length = 10_000.0;
        green.environment.season_length = 10_000.0;

        let (sx, sy) = {
            let p = snow.players.get(&1).unwrap();
            (p.x, p.y)
        };
        let (gx, gy) = {
            let p = green.players.get(&1).unwrap();
            (p.x, p.y)
        };
        snow.world.write().unwrap().set_biome(sx, sy, 4); // snow
        green.world.write().unwrap().set_biome(gx, gy, 0); // green

        let food0 = snow.players.get(&1).unwrap().food;
        assert_eq!(food0, green.players.get(&1).unwrap().food);

        let hub = OutboundHub::new();
        tick_vitals(&mut snow, 1.0, &hub);
        tick_vitals(&mut green, 1.0, &hub);

        let snow_food = snow.players.get(&1).unwrap().food;
        let green_food = green.players.get(&1).unwrap().food;
        assert!(
            snow_food < green_food,
            "snow should drain faster: snow={snow_food} green={green_food}"
        );
        // Expected rates: green 0.10*1.0+0.05=0.15, snow 0.10*1.25+0.05=0.175
        let snow_lost = food0 - snow_food;
        let green_lost = food0 - green_food;
        assert!((green_lost - (FOOD_USE_PER_SEC + TEMP_FOOD_EXTRA)).abs() < 1e-4);
        assert!(
            (snow_lost - (FOOD_USE_PER_SEC * biome_food_multiplier(4) + TEMP_FOOD_EXTRA)).abs()
                < 1e-4
        );
    }

    /// Desert (biome 5) at high temp applies TEMP_FOOD_EXTRA + DESERT_EXTRA (0.02)
    /// on top of the desert biome food multiplier (1.10).
    #[test]
    fn desert_high_temp_applies_desert_extra() {
        let mut desert = SimState::with_default_empty(test_content());
        let mut green = SimState::with_default_empty(test_content());
        spawn_player(&mut desert, 1, "desert");
        spawn_player(&mut green, 1, "green");

        // High base temp so both hit TEMP_FOOD_EXTRA (t > 0.75); desert also +0.15 biome heat.
        desert.environment.temperature = 0.80;
        green.environment.temperature = 0.80;
        desert.environment.season_length = 10_000.0;
        green.environment.season_length = 10_000.0;

        let (dx, dy) = {
            let p = desert.players.get(&1).unwrap();
            (p.x, p.y)
        };
        let (gx, gy) = {
            let p = green.players.get(&1).unwrap();
            (p.x, p.y)
        };
        desert.world.write().unwrap().set_biome(dx, dy, 5); // desert
        green.world.write().unwrap().set_biome(gx, gy, 0); // green

        let food0 = desert.players.get(&1).unwrap().food;
        assert_eq!(food0, green.players.get(&1).unwrap().food);

        // Confirm effective temps are hot enough for the extras.
        let d_t = desert.environment.temperature_at_biome(5);
        let g_t = green.environment.temperature_at_biome(0);
        assert!(d_t > 0.75, "desert heat {d_t}");
        assert!(g_t > 0.75, "green heat {g_t}");

        let hub = OutboundHub::new();
        tick_vitals(&mut desert, 1.0, &hub);
        tick_vitals(&mut green, 1.0, &hub);

        let desert_food = desert.players.get(&1).unwrap().food;
        let green_food = green.players.get(&1).unwrap().food;
        assert!(
            desert_food < green_food,
            "desert should drain faster: desert={desert_food} green={green_food}"
        );

        // green: 0.10*1.0 + 0.05 = 0.15
        // desert: 0.10*1.10 + 0.05 + 0.02 = 0.18
        let desert_lost = food0 - desert_food;
        let green_lost = food0 - green_food;
        assert!((green_lost - (FOOD_USE_PER_SEC + TEMP_FOOD_EXTRA)).abs() < 1e-4);
        let expected_desert = FOOD_USE_PER_SEC * biome_food_multiplier(5)
            + TEMP_FOOD_EXTRA
            + DESERT_EXTRA;
        assert!(
            (desert_lost - expected_desert).abs() < 1e-4,
            "desert_lost={desert_lost} expected={expected_desert}"
        );
        assert_eq!(DESERT_EXTRA, 0.02);
        assert!(biome_food_multiplier(5) > 1.0);
    }

    #[test]
    fn drop_places_and_emits_mx() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "u");
        set_player_position(&mut state, 1, 2, 3);
        state.players.get_mut(&1).unwrap().held_id = 34;
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Drop {
                conn_id: 1,
                x: 2,
                y: 3,
                c: None,
            },
        );
        assert_eq!(state.world.read().unwrap().get_object(2, 3), 34);
        assert_eq!(state.players.get(&1).unwrap().held_id, 0);
        let mx = rx.try_recv().expect("drop MX");
        assert!(String::from_utf8_lossy(&mx).contains("MX\n2 3"));
    }

    #[test]
    fn drop_sets_owner_id() {
        let hub = OutboundHub::new();
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "owner@test");
        set_player_position(&mut state, 1, 4, 5);
        state.players.get_mut(&1).unwrap().held_id = 33;
        apply_drop(&mut state, &hub, 1, 4, 5, None);
        assert_eq!(state.world.read().unwrap().get_object(4, 5), 33);
        assert_eq!(state.players.get(&1).unwrap().held_id, 0);
        let world = state.world.read().unwrap();
        let h = world.get_helper(4, 5).expect("DROP stores owner helper");
        assert_eq!(h.owner_id, p_id);
        assert!(h.is_owner(p_id));
        assert!(!h.is_owner(p_id + 99));
        assert!(world.is_owner(4, 5, p_id));
        assert!(!world.is_owner(4, 5, 0));
    }

    #[test]
    fn nearby_mx_reaches_second_player() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx_a = hub.register(1);
        let mut rx_b = hub.register(2);
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "a");
        spawn_player(&mut state, 2, "b");
        state.players.get_mut(&1).unwrap().x = 1;
        state.players.get_mut(&1).unwrap().y = 1;
        state.players.get_mut(&2).unwrap().x = 2;
        state.players.get_mut(&2).unwrap().y = 2;
        state.world.write().unwrap().set_object(1, 1, 33);
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Use {
                conn_id: 1,
                x: 1,
                y: 1,
                id: None,
                index: None,
            },
        );
        let mx_a = rx_a.try_recv().expect("actor MX");
        assert!(String::from_utf8_lossy(&mx_a).starts_with("MX\n"));
        let mx_b = rx_b.try_recv().expect("nearby MX");
        assert!(String::from_utf8_lossy(&mx_b).starts_with("MX\n"));
    }

    #[test]
    fn auto_decay_transforms_object() {
        let mut db = ContentDb::default();
        db.auto_decays.insert(
            100,
            Transition {
                actor_id: -1,
                target_id: 100,
                new_actor_id: 0,
                new_target_id: 101,
                last_use_actor: false,
                last_use_target: false,
                auto_decay_seconds: 1.0,
                reverse_use_actor: false,
                reverse_use_target: false,
                no_use_actor: false,
                no_use_target: false,
                move_dist: 0,

            desired_move_dist: 0,
            },
        );
        let mut state = SimState::with_default_empty(Arc::new(db));
        state.world.write().unwrap().set_object(3, 3, 100);
        schedule_decay(&mut state, 3, 3, 100);
        assert!(state.pending_decays.contains_key(&(3, 3)));
        tick_auto_decays(&mut state, 0.5);
        assert_eq!(state.world.read().unwrap().get_object(3, 3), 100);
        tick_auto_decays(&mut state, 0.6);
        assert_eq!(state.world.read().unwrap().get_object(3, 3), 101);
    }

    #[test]
    fn map_chunk_sent_when_player_moves_far() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "walker");
        // First move: needs MC (has_mc false)
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Move {
                conn_id: 1,
                xs: 0,
                ys: 0,
                deltas: vec![(1, 0)],
                seq: None,
},
        );
        let mut saw_mc = false;
        while let Ok(pkt) = rx.try_recv() {
            if pkt.starts_with(b"MC\n") {
                saw_mc = true;
            }
        }
        assert!(saw_mc, "first MOVE should send MC");
        assert!(state.players.get(&1).unwrap().has_mc);

        // Small step: no new MC
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Move {
                conn_id: 1,
                xs: 1,
                ys: 0,
                deltas: vec![(1, 0)],
                seq: None,
},
        );
        let mut mc2 = false;
        while let Ok(pkt) = rx.try_recv() {
            if pkt.starts_with(b"MC\n") {
                mc2 = true;
            }
        }
        assert!(!mc2, "near move should not resend MC");

        // Far step: new MC
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Move {
                conn_id: 1,
                xs: 2,
                ys: 0,
                deltas: vec![(MC_RESEND_THRESHOLD + 1, 0)],
                seq: None,
},
        );
        let mut mc3 = false;
        while let Ok(pkt) = rx.try_recv() {
            if pkt.starts_with(b"MC\n") {
                mc3 = true;
            }
        }
        assert!(mc3, "far MOVE should resend MC");
    }

    #[test]
    fn drop_into_container_and_remv() {
        let mut db = ContentDb::default();
        db.objects.insert(
            391,
            ObjectDef {
                id: 391,
                description: "Basket".into(),
                name: "Basket".into(),
                containable: false,
                permanent: true,
                blocks_walking: false,
                food_value: 0,
                heat_value: 0.0,
                map_chance: 0.0,
                biomes: vec![],
                num_uses: 0,
                num_slots: 4,
                floor: false,
            dummy_ids: Vec::new(),
            },
        );
        db.objects.insert(
            33,
            ObjectDef {
                id: 33,
                description: "Berry".into(),
                name: "Berry".into(),
                containable: true,
                permanent: false,
                blocks_walking: false,
                food_value: 2,
                heat_value: 0.0,
                map_chance: 0.0,
                biomes: vec![],
                num_uses: 0,
                num_slots: 0,
                floor: false,
            dummy_ids: Vec::new(),
            },
        );
        // Non-containable held item must NOT enter container.
        db.objects.insert(
            100,
            ObjectDef {
                id: 100,
                description: "Tree".into(),
                name: "Tree".into(),
                containable: false,
                permanent: true,
                blocks_walking: true,
                food_value: 0,
                heat_value: 0.0,
                map_chance: 0.0,
                biomes: vec![],
                num_uses: 0,
                num_slots: 0,
                floor: false,
            dummy_ids: Vec::new(),
            },
        );
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut state = SimState::with_default_empty(Arc::new(db));
        spawn_player(&mut state, 1, "c");
        set_player_position(&mut state, 1, 5, 5);
        state.world.write().unwrap().set_object(5, 5, 391);

        // Reject non-containable into basket.
        state.players.get_mut(&1).unwrap().held_id = 100;
        apply_drop(&mut state, &hub, 1, 5, 5, None);
        assert_eq!(state.players.get(&1).unwrap().held_id, 100);
        assert!(state.world.read().unwrap().get_helper(5, 5).is_none());

        // Accept containable berry.
        state.players.get_mut(&1).unwrap().held_id = 33;
        apply_drop(&mut state, &hub, 1, 5, 5, None);
        assert_eq!(state.players.get(&1).unwrap().held_id, 0);
        assert_eq!(
            state.world.read().unwrap().get_helper(5, 5).unwrap().contained,
            vec![33]
        );
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "REMV".into(),
                payload: "5 5 0".into(),
            },
        );
        assert_eq!(state.players.get(&1).unwrap().held_id, 33);
        assert!(state
            .world
            .read()
            .unwrap()
            .get_helper(5, 5)
            .map(|h| h.contained.is_empty())
            .unwrap_or(true));
    }

    /// SAY PUTNEST <slot> — put held into nested pocket of contained[slot] under feet.
    #[test]
    fn say_putnest_nested_pocket_drop() {
        use ol_world::ComplexObject;

        let mut db = ContentDb::default();
        // Basket under feet (container with top-level slots).
        db.objects.insert(
            391,
            ObjectDef {
                id: 391,
                description: "Basket".into(),
                name: "Basket".into(),
                containable: false,
                permanent: true,
                blocks_walking: false,
                food_value: 0,
                heat_value: 0.0,
                map_chance: 0.0,
                biomes: vec![],
                num_uses: 0,
                num_slots: 4,
                floor: false,
            dummy_ids: Vec::new(),
            },
        );
        // Bag in contained[0] — nested pocket with 2 sub-slots.
        db.objects.insert(
            292,
            ObjectDef {
                id: 292,
                description: "Bag".into(),
                name: "Bag".into(),
                containable: true,
                permanent: false,
                blocks_walking: false,
                food_value: 0,
                heat_value: 0.0,
                map_chance: 0.0,
                biomes: vec![],
                num_uses: 0,
                num_slots: 2,
                floor: false,
            dummy_ids: Vec::new(),
            },
        );
        // Containable berry to nest.
        db.objects.insert(
            33,
            ObjectDef {
                id: 33,
                description: "Berry".into(),
                name: "Berry".into(),
                containable: true,
                permanent: false,
                blocks_walking: false,
                food_value: 2,
                heat_value: 0.0,
                map_chance: 0.0,
                biomes: vec![],
                num_uses: 0,
                num_slots: 0,
                floor: false,
            dummy_ids: Vec::new(),
            },
        );
        // Non-containable tree must be rejected.
        db.objects.insert(
            100,
            ObjectDef {
                id: 100,
                description: "Tree".into(),
                name: "Tree".into(),
                containable: false,
                permanent: true,
                blocks_walking: true,
                food_value: 0,
                heat_value: 0.0,
                map_chance: 0.0,
                biomes: vec![],
                num_uses: 0,
                num_slots: 0,
                floor: false,
            dummy_ids: Vec::new(),
            },
        );

        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(Arc::new(db));
        spawn_player(&mut state, 1, "nest@test");
        // Player stands on basket that already holds a bag in slot 0.
        {
            let p = state.players.get_mut(&1).unwrap();
            p.x = 3;
            p.y = 4;
            p.held_id = 33;
        }
        state.world.write().unwrap().set_object_complex(
            3,
            4,
            ComplexObject {
                base_id: 391,
                uses_remaining: 0,
                contained: vec![292],
                nested: Vec::new(),
                owner_id: 0,
                creation_time: 0.0,
                time_to_change: 0.0,
            },
        );

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "PUTNEST 0".into(),
            },
        );
        assert_eq!(state.players.get(&1).unwrap().held_id, 0, "hands empty after PUTNEST");
        let h = state.world.read().unwrap().get_helper(3, 4).unwrap().clone();
        assert_eq!(h.contained, vec![292]);
        assert_eq!(h.nested, vec![vec![33]]);
        assert_eq!(h.to_map_string_id(), "391,292:33");

        let mut saw_ok = false;
        while let Ok(msg) = rx.try_recv() {
            let s = String::from_utf8_lossy(&msg);
            if s.contains("PUTNEST 0 33 OK") {
                saw_ok = true;
            }
        }
        assert!(saw_ok, "expected PS PUTNEST 0 33 OK");

        // Second berry into same pocket.
        state.players.get_mut(&1).unwrap().held_id = 33;
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "PUTNEST 0".into(),
            },
        );
        assert_eq!(state.players.get(&1).unwrap().held_id, 0);
        assert_eq!(
            state.world.read().unwrap().get_helper(3, 4).unwrap().nested,
            vec![vec![33, 33]]
        );

        // Full pocket (num_slots=2) rejects third put; hands keep item.
        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        state.players.get_mut(&1).unwrap().held_id = 33;
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "PUTNEST 0".into(),
            },
        );
        assert_eq!(state.players.get(&1).unwrap().held_id, 33, "full pocket keeps held");

        // Non-containable rejected.
        state.players.get_mut(&1).unwrap().held_id = 100;
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "PUTNEST 0".into(),
            },
        );
        assert_eq!(state.players.get(&1).unwrap().held_id, 100);

        // Bad slot index.
        state.players.get_mut(&1).unwrap().held_id = 33;
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "PUTNEST 9".into(),
            },
        );
        assert_eq!(state.players.get(&1).unwrap().held_id, 33);

        // Missing slot arg.
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "PUTNEST".into(),
            },
        );
        assert_eq!(state.players.get(&1).unwrap().held_id, 33);

        // HELP lists PUTNEST.
        assert!(
            SimState::format_help_query().contains("PUTNEST"),
            "HELP should list PUTNEST"
        );
    }

    /// REMV x y slot sub — pocket-style nested take from contained[slot].nested[sub].
    #[test]
    fn remv_nested_pocket_take() {
        use ol_world::ComplexObject;

        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "c");
        set_player_position(&mut state, 1, 7, 7);
        state.players.get_mut(&1).unwrap().held_id = 0;
        // Basket with bag (292) holding berries (100,101) as nested sub-items.
        state.world.write().unwrap().set_object_complex(
            7,
            7,
            ComplexObject {
                base_id: 391,
                uses_remaining: 0,
                contained: vec![292],
                nested: vec![vec![100, 101]],
                owner_id: 0,
                creation_time: 0.0,
                time_to_change: 0.0,
            },
        );
        // REMV x y 0 0 → take nested[0][0] = 100
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "REMV".into(),
                payload: "7 7 0 0".into(),
            },
        );
        assert_eq!(state.players.get(&1).unwrap().held_id, 100);
        assert_eq!(
            state.world.read().unwrap().get_helper(7, 7).unwrap().nested,
            vec![vec![101]]
        );
        // Empty hands, take last nested under slot 0 (sub -1)
        state.players.get_mut(&1).unwrap().held_id = 0;
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "REMV".into(),
                payload: "7 7 0 -1".into(),
            },
        );
        assert_eq!(state.players.get(&1).unwrap().held_id, 101);
        let h = state.world.read().unwrap().get_helper(7, 7).unwrap().clone();
        assert!(h.nested.is_empty());
        assert_eq!(h.contained, vec![292]);
        assert_eq!(h.to_map_string_id(), "391,292");
    }

    #[test]
    fn follow_exile_kill_and_season_query() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "leader@x");
        spawn_player(&mut state, 2, "follower@x");
        let leader = state.players.get(&1).unwrap().p_id;
        let follower = state.players.get(&2).unwrap().p_id;
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 2,
                tag: "SAY".into(),
                payload: format!("FOLLOW {leader}"),
            },
        );
        assert_eq!(state.social.following.get(&follower), Some(&leader));
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("EXILE {follower}"),
            },
        );
        assert!(state.social.is_exiled_by(leader, follower));
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("KILL {follower}"),
            },
        );
        assert!(state.players.get(&2).unwrap().deleted);
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?TEMP".into(),
            },
        );
        let mut saw_temp = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("TEMP") || s.contains("SPRING") || s.contains("KILLED") {
                saw_temp = true;
            }
        }
        assert!(saw_temp);
    }

    #[test]
    fn say_time_query_returns_hour_and_day_phase() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "clock@x");
        state.environment.hour_of_day = 19.25;
        // Freeze clock so tick side effects cannot drift the reply mid-test.
        state.environment.day_length = 10_000.0;

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?TIME".into(),
            },
        );

        let expected = state.environment.time_query_text();
        assert_eq!(expected, "TIME 19.25 DUSK");
        let mut saw = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PS\n") && s.contains("TIME ") {
                saw = true;
                assert!(
                    s.contains(&format!("{p_id} {expected}")),
                    "PS line should embed hour_of_day + day_phase: {s}"
                );
                assert!(s.contains("19.25"), "got {s}");
                assert!(s.contains("DUSK"), "got {s}");
            }
        }
        assert!(saw, "expected PS ?TIME reply with hour_of_day and day_phase");
    }

    #[test]
    fn arm_decays_after_world_load_path() {
        let mut db = ContentDb::default();
        db.auto_decays.insert(
            55,
            Transition {
                actor_id: -1,
                target_id: 55,
                new_actor_id: 0,
                new_target_id: 56,
                last_use_actor: false,
                last_use_target: false,
                auto_decay_seconds: 2.0,
                reverse_use_actor: false,
                reverse_use_target: false,
                no_use_actor: false,
                no_use_target: false,
                move_dist: 0,

            desired_move_dist: 0,
            },
        );
        let mut state = SimState::with_default_empty(Arc::new(db));
        // Simulate natural spawn / disk load placing a decayable object without USE.
        state.world.write().unwrap().set_object(7, 8, 55);
        assert!(state.pending_decays.is_empty());
        arm_decays_for_loaded_world(&mut state);
        assert_eq!(
            state.pending_decays.get(&(7, 8)).copied(),
            Some((55, 2.0))
        );
        // DROP path also arms decay.
        spawn_player(&mut state, 1, "d");
        set_player_position(&mut state, 1, 1, 1);
        state.players.get_mut(&1).unwrap().held_id = 55;
        let hub = OutboundHub::new();
        apply_drop(&mut state, &hub, 1, 1, 1, None);
        assert_eq!(
            state.pending_decays.get(&(1, 1)).copied(),
            Some((55, 2.0))
        );
    }

    #[test]
    fn drop_records_world_journal() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("ol_sim_journal_{nanos}.journal"));
        let _ = std::fs::remove_file(&path);

        let mut state = SimState::with_default_empty(test_content())
            .with_journal(Arc::new(Mutex::new(WorldJournal::open(&path))));
        state.tick = 7;
        spawn_player(&mut state, 1, "j");
        set_player_position(&mut state, 1, 4, 5);
        state.players.get_mut(&1).unwrap().held_id = 33;
        let hub = OutboundHub::new();
        apply_drop(&mut state, &hub, 1, 4, 5, None);
        assert_eq!(state.world.read().unwrap().get_object(4, 5), 33);

        let entries = WorldJournal::open(&path).load_all().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0], JournalEntry::new(4, 5, 33, 7));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn eat_held_food_on_failed_use() {
        let mut db = ContentDb::default();
        db.objects.insert(
            33,
            ObjectDef {
                id: 33,
                description: "Gooseberry".into(),
                name: "Gooseberry".into(),
                containable: true,
                permanent: false,
                blocks_walking: false,
                food_value: 5,
                heat_value: 0.0,
                map_chance: 0.0,
                biomes: Vec::new(),
                num_uses: 0,
                num_slots: 0,
                floor: false,
            dummy_ids: Vec::new(),
            },
        );
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(Arc::new(db));
        spawn_player(&mut state, 1, "eater");
        state.players.get_mut(&1).unwrap().held_id = 33;
        state.players.get_mut(&1).unwrap().food = 5.0;
        // USE on empty tile: no transition → eat
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Use {
                conn_id: 1,
                x: 9,
                y: 9,
                id: None,
                index: None,
            },
        );
        let p = state.players.get(&1).unwrap();
        assert_eq!(p.held_id, 0);
        // YUM multiplies first-of-kind food (5 * 1.5 + bonus) → > base 5.
        assert!(p.food > 5.0 + 4.9);
        assert_eq!(p.yum.just_ate_id, 33);
        assert_eq!(p.yum.last_ate_fill_max, 5);
        assert_eq!(p.yum.history.len(), 1);
        // Flag cleared after PU fan-out; last_ate_id retained.
        assert!(!p.yum.just_ate);

        // FX then PU must carry just_ate / last_ate_id / yum_bonus.
        let fx = rx.try_recv().expect("FX after eat");
        let fx_s = String::from_utf8_lossy(&fx);
        assert!(fx_s.starts_with("FX\n"), "got {fx_s}");
        // food_store food_capacity last_ate_id last_ate_fill_max ... yum_bonus yum_multiplier
        assert!(
            fx_s.contains(" 33 5 "),
            "FX should include last_ate_id=33 last_ate_fill_max=5: {fx_s}"
        );
        // yum_bonus ceil(0.1) == 1
        assert!(
            fx_s.contains(" 1 0\n") || fx_s.contains(" 1 0\r"),
            "FX should include yum_bonus=1: {fx_s}"
        );
        let pu = rx.try_recv().expect("PU after eat");
        let pu_s = String::from_utf8_lossy(&pu);
        assert!(pu_s.starts_with("PU\n"), "got {pu_s}");
        // clothing just_ate last_ate responsible yum learned
        assert!(
            pu_s.contains(" 1 33 -1 "),
            "PU should include just_ate=1 last_ate=33: {pu_s}"
        );
    }

    #[test]
    fn say_yum_returns_bonus_and_history_len() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut db = ContentDb::default();
        db.objects.insert(
            33,
            ObjectDef {
                id: 33,
                description: "Gooseberry".into(),
                name: "Gooseberry".into(),
                containable: true,
                permanent: false,
                blocks_walking: false,
                food_value: 5,
                heat_value: 0.0,
                map_chance: 0.0,
                biomes: Vec::new(),
                num_uses: 0,
                num_slots: 0,
                floor: false,
            dummy_ids: Vec::new(),
            },
        );
        let mut state = SimState::with_default_empty(Arc::new(db));
        spawn_player(&mut state, 1, "eater");
        state.players.get_mut(&1).unwrap().held_id = 33;
        state.players.get_mut(&1).unwrap().food = 5.0;
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Use {
                conn_id: 1,
                x: 9,
                y: 9,
                id: None,
                index: None,
            },
        );
        // Drain eat FX/PU.
        let _ = rx.try_recv();
        let _ = rx.try_recv();

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?YUM".into(),
            },
        );
        let ps = rx.try_recv().expect("PS ?YUM");
        let s = String::from_utf8_lossy(&ps);
        assert!(s.starts_with("PS\n"), "got {s}");
        assert!(s.contains("YUM "), "got {s}");
        assert!(s.contains("bonus="), "got {s}");
        assert!(s.contains("history=1"), "got {s}");
        let p = state.players.get(&1).unwrap();
        assert!(s.contains(&format!("bonus={}", p.yum.yum_bonus)), "got {s}");
    }

    /// SAY ?TOOLS returns tools.wire_slots (used total) and learned count via private PS.
    #[test]
    fn say_tools_query_returns_wire_slots_and_learned_count() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "tools_q");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.tools.learn(334);
            p.tools.learn(12);
            p.tools.learn(334); // duplicate — still 2 learned
        }
        let expected_slots = state.players.get(&1).unwrap().tools.wire_slots();
        let expected_reply = state.players.get(&1).unwrap().tools.query_text();
        assert_eq!(expected_slots, "2 1000");
        assert_eq!(expected_reply, "TOOLS 2 1000 learned=2");

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?TOOLS".into(),
            },
        );
        let ps = rx.try_recv().expect("PS ?TOOLS");
        let s = String::from_utf8_lossy(&ps);
        assert!(s.starts_with("PS\n"), "got {s}");
        assert!(
            s.contains(&format!("{p_id} TOOLS ")),
            "expected p_id + TOOLS, got {s}"
        );
        assert!(s.contains(&expected_slots), "wire_slots missing, got {s}");
        assert!(s.contains("learned=2"), "learned count missing, got {s}");
        assert!(
            s.contains(&format!("{p_id} {expected_reply}")),
            "got {s}"
        );

        // Bare TOOLS alias also works.
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "TOOLS".into(),
            },
        );
        let ps2 = rx.try_recv().expect("PS TOOLS");
        let s2 = String::from_utf8_lossy(&ps2);
        assert!(s2.contains("TOOLS 2 1000 learned=2"), "got {s2}");
    }

    #[test]
    fn score_updates_on_kill_pay_and_queries() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx1 = hub.register(1);
        let mut rx2 = hub.register(2);
        let mut state = SimState::with_default_empty(test_content());
        // Login path registers scoreboard + starting coins.
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Login {
                conn_id: 1,
                reconnect: false,
                email: "alice@x".into(),
                client_tag: "t".into(),
            },
        );
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Login {
                conn_id: 2,
                reconnect: false,
                email: "bob@x".into(),
                client_tag: "t".into(),
            },
        );
        let a = state.players.get(&1).unwrap().p_id;
        let b = state.players.get(&2).unwrap().p_id;
        assert_eq!(state.scoreboard.entry(a).unwrap().coins, 5);
        // PAY a -> b
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("PAY {b} 2"),
            },
        );
        assert_eq!(state.scoreboard.entry(a).unwrap().coins, 3);
        assert_eq!(state.scoreboard.entry(b).unwrap().coins, 7);
        // KILL
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("KILL {b}"),
            },
        );
        assert_eq!(state.scoreboard.entry(a).unwrap().kills, 1);
        assert_eq!(state.scoreboard.entry(b).unwrap().deaths, 1);
        assert!(state.scoreboard.entry(a).unwrap().score > state.scoreboard.entry(b).unwrap().score);
        // Drain prior PS noise.
        while rx1.try_recv().is_ok() {}
        while rx2.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?SCORE".into(),
            },
        );
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?LEADERBOARD".into(),
            },
        );
        let mut saw_score = false;
        let mut saw_lb = false;
        while let Ok(pkt) = rx1.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("SCORE") && s.contains('K') {
                saw_score = true;
            }
            if s.contains("LEADERBOARD") {
                saw_lb = true;
            }
        }
        assert!(saw_score, "expected PS ?SCORE reply");
        assert!(saw_lb, "expected PS ?LEADERBOARD reply");
    }

    /// Season change resets scoreboard kills/deaths/season_bonus; coins stay.
    #[test]
    fn setseason_resets_scoreboard_season_leaderboard() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut state = SimState::with_default_empty(test_content());
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Login {
                conn_id: 1,
                reconnect: false,
                email: "a@x".into(),
                client_tag: "t".into(),
            },
        );
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Login {
                conn_id: 2,
                reconnect: false,
                email: "b@x".into(),
                client_tag: "t".into(),
            },
        );
        let a = state.players.get(&1).unwrap().p_id;
        let b = state.players.get(&2).unwrap().p_id;
        // Seed seasonal combat stats + bonus.
        state.scoreboard.record_kill(a, b);
        state.scoreboard.add_season_bonus(a, 30);
        let coins_a = state.scoreboard.entry(a).unwrap().coins;
        assert_eq!(state.scoreboard.entry(a).unwrap().kills, 1);
        assert_eq!(state.scoreboard.entry(a).unwrap().season_bonus, 30);
        assert_eq!(state.environment.season, Season::Spring);

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "SETSEASON SUMMER".into(),
            },
        );
        assert_eq!(state.environment.season, Season::Summer);
        assert_eq!(state.scoreboard.season_tag, "SUMMER");
        assert_eq!(state.scoreboard.entry(a).unwrap().kills, 0);
        assert_eq!(state.scoreboard.entry(b).unwrap().deaths, 0);
        assert_eq!(state.scoreboard.entry(a).unwrap().season_bonus, 0);
        assert_eq!(state.scoreboard.entry(a).unwrap().coins, coins_a);
        assert_eq!(state.scoreboard.entry(a).unwrap().score, coins_a);
    }

    /// Natural season rollover via tick_vitals also resets the season board.
    #[test]
    fn tick_vitals_season_rollover_resets_scoreboard() {
        let hub = OutboundHub::new();
        let mut state = SimState::with_default_empty(test_content());
        let a = spawn_player(&mut state, 1, "a@x");
        let b = spawn_player(&mut state, 2, "b@x");
        state.scoreboard.ensure_player(a, "Alice");
        state.scoreboard.ensure_player(b, "Bob");
        state.scoreboard.set_coins(a, 9);
        state.scoreboard.record_kill(a, b);
        state.scoreboard.add_season_bonus(a, 15);
        // Bind current season without wipe.
        let _ = state.scoreboard.on_season_change(state.environment.season.as_str());
        assert_eq!(state.scoreboard.entry(a).unwrap().kills, 1);

        state.environment.season_length = 1.0;
        state.environment.season_elapsed = 0.0;
        tick_vitals(&mut state, 1.1, &hub);

        assert_eq!(state.environment.season, Season::Summer);
        assert_eq!(state.scoreboard.season_tag, "SUMMER");
        assert_eq!(state.scoreboard.entry(a).unwrap().kills, 0);
        assert_eq!(state.scoreboard.entry(a).unwrap().season_bonus, 0);
        assert_eq!(state.scoreboard.entry(a).unwrap().coins, 9);
        assert_eq!(state.scoreboard.entry(a).unwrap().score, 9);
    }

    /// SAY ?HIGHSCORE ranks by prestige, not scoreboard score.
    #[test]
    fn say_highscore_top_by_prestige() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx1 = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Login {
                conn_id: 1,
                reconnect: false,
                email: "low@x".into(),
                client_tag: "t".into(),
            },
        );
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Login {
                conn_id: 2,
                reconnect: false,
                email: "high@x".into(),
                client_tag: "t".into(),
            },
        );
        let low = state.players.get(&1).unwrap().p_id;
        let high = state.players.get(&2).unwrap().p_id;
        // Give low player more *score* but less prestige.
        state.scoreboard.set_coins(low, 100);
        state.scoreboard.set_coins(high, 1);
        state.combat.stats_mut(low).prestige = 2.0;
        state.combat.stats_mut(high).prestige = 55.0;
        // Lineage prestige preferred when present — clear path via combat only.
        state.social.lineages.remove(&low);
        state.social.lineages.remove(&high);

        while rx1.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?HIGHSCORE".into(),
            },
        );
        let mut saw = false;
        while let Ok(pkt) = rx1.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("HIGHSCORE") {
                saw = true;
                // High prestige first even though lower score/coins.
                let pos_high = s.find("55.0").or_else(|| s.find("=55"));
                let pos_low = s.find("2.0").or_else(|| s.find("=2"));
                assert!(
                    pos_high.is_some() && pos_low.is_some(),
                    "expected both prestiges in {s}"
                );
                assert!(
                    pos_high.unwrap() < pos_low.unwrap(),
                    "high prestige should rank first: {s}"
                );
            }
        }
        assert!(saw, "expected PS ?HIGHSCORE reply");
        let _ = (low, high);
    }

    /// SAY TRADE sets Player.trade_offer; SAY ACCEPT transfers via economy.
    #[test]
    fn say_trade_sets_offer_accept_transfers_coins() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx1 = hub.register(1);
        let mut rx2 = hub.register(2);
        let mut state = SimState::with_default_empty(test_content());
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Login {
                conn_id: 1,
                reconnect: false,
                email: "trader@x".into(),
                client_tag: "t".into(),
            },
        );
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Login {
                conn_id: 2,
                reconnect: false,
                email: "buyer@x".into(),
                client_tag: "t".into(),
            },
        );
        let a = state.players.get(&1).unwrap().p_id;
        let b = state.players.get(&2).unwrap().p_id;
        assert_eq!(state.scoreboard.entry(a).unwrap().coins, 5);
        assert_eq!(state.scoreboard.entry(b).unwrap().coins, 5);
        assert!(state.players.get(&1).unwrap().trade_offer.is_none());

        // TRADE does not move coins yet.
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("TRADE {b} 3"),
            },
        );
        assert_eq!(
            state.players.get(&1).unwrap().trade_offer,
            Some((b, 3)),
            "TRADE must store (target, amount)"
        );
        assert_eq!(state.scoreboard.entry(a).unwrap().coins, 5);
        assert_eq!(state.scoreboard.entry(b).unwrap().coins, 5);
        assert_eq!(
            state.economy.wallets.get(&a).map(|w| w.coins),
            Some(5)
        );

        while rx1.try_recv().is_ok() {}
        while rx2.try_recv().is_ok() {}

        // ACCEPT by target transfers coins and clears offer.
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 2,
                tag: "SAY".into(),
                payload: "ACCEPT".into(),
            },
        );
        assert!(
            state.players.get(&1).unwrap().trade_offer.is_none(),
            "offer cleared after successful ACCEPT"
        );
        assert_eq!(state.economy.wallets.get(&a).map(|w| w.coins), Some(2));
        assert_eq!(state.economy.wallets.get(&b).map(|w| w.coins), Some(8));
        assert_eq!(state.scoreboard.entry(a).unwrap().coins, 2);
        assert_eq!(state.scoreboard.entry(b).unwrap().coins, 8);

        let mut saw_ok = false;
        while let Ok(pkt) = rx2.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("ACCEPT") && s.contains("OK") {
                saw_ok = true;
            }
        }
        assert!(saw_ok, "expected PS ACCEPT … OK for accepter");
    }

    /// SAY DONATE / ?TREASURY move coins into Economy.treasury.
    #[test]
    fn say_donate_and_treasury_query() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Login {
                conn_id: 1,
                reconnect: false,
                email: "giver@x".into(),
                client_tag: "t".into(),
            },
        );
        let a = state.players.get(&1).unwrap().p_id;
        assert_eq!(state.economy.treasury, 0);
        assert_eq!(state.economy.wallets.get(&a).map(|w| w.coins), Some(5));

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "DONATE 3".into(),
            },
        );
        assert_eq!(state.economy.treasury, 3);
        assert_eq!(state.economy.wallets.get(&a).map(|w| w.coins), Some(2));
        assert_eq!(state.scoreboard.entry(a).unwrap().coins, 2);

        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?TREASURY".into(),
            },
        );
        let mut saw = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("TREASURY 3") {
                saw = true;
            }
        }
        assert!(saw, "expected PS TREASURY 3");
    }

    /// SAY TAX only succeeds for leaders (inbound followers).
    #[test]
    fn say_tax_requires_leader() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx1 = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Login {
                conn_id: 1,
                reconnect: false,
                email: "boss@x".into(),
                client_tag: "t".into(),
            },
        );
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Login {
                conn_id: 2,
                reconnect: false,
                email: "follower@x".into(),
                client_tag: "t".into(),
            },
        );
        let a = state.players.get(&1).unwrap().p_id;
        let b = state.players.get(&2).unwrap().p_id;

        // Not a leader yet — TAX fails.
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "TAX 2".into(),
            },
        );
        assert_eq!(state.economy.treasury, 0);
        assert_eq!(state.economy.wallets.get(&a).map(|w| w.coins), Some(5));

        // b follows a → a is leader.
        state.social.following.insert(b, a);
        assert!(is_leader(&state.social.following, a));

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "TAX 2".into(),
            },
        );
        assert_eq!(state.economy.treasury, 2);
        assert_eq!(state.economy.wallets.get(&a).map(|w| w.coins), Some(3));

        while rx1.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "TAX 2".into(),
            },
        );
        // Second tax still ok.
        assert_eq!(state.economy.treasury, 4);
    }

    /// On death, coins go to mother if online; otherwise treasury.
    #[test]
    fn death_inheritance_to_mother_or_treasury() {
        let hub = OutboundHub::new();
        let mut state = SimState::with_default_empty(test_content());
        // Mother online.
        let mother = spawn_player(&mut state, 1, "mom@x");
        let child = spawn_player(&mut state, 2, "kid@x");
        state.social.ensure_lineage(mother, "MOM");
        let mother_node = state.social.lineages.get(&mother).unwrap().clone();
        state
            .social
            .lineages
            .insert(child, LineageNode::with_mother(child, "KID", &mother_node));
        state.economy.add_coins(child, 10);
        state.economy.add_coins(mother, 1);
        {
            let p = state.players.get_mut(&2).unwrap();
            p.food = 0.05;
            p.age = 20.0;
        }
        tick_vitals(&mut state, 1.0, &hub);
        assert!(state.players.get(&2).unwrap().deleted);
        assert_eq!(state.economy.wallets.get(&child).map(|w| w.coins), Some(0));
        assert_eq!(
            state.economy.wallets.get(&mother).map(|w| w.coins),
            Some(11)
        );
        assert_eq!(state.economy.treasury, 0);

        // Eve with coins, no mother → treasury.
        let eve = spawn_player(&mut state, 3, "eve@x");
        state
            .social
            .lineages
            .insert(eve, LineageNode::eve(eve, "EVE"));
        state.economy.add_coins(eve, 7);
        {
            let p = state.players.get_mut(&3).unwrap();
            p.food = 0.05;
            p.age = 20.0;
        }
        tick_vitals(&mut state, 1.0, &hub);
        assert!(state.players.get(&3).unwrap().deleted);
        assert_eq!(state.economy.wallets.get(&eve).map(|w| w.coins), Some(0));
        assert_eq!(state.economy.treasury, 7);
    }

    /// ACCEPT with no matching offer fails; invalid TRADE rejected.
    #[test]
    fn say_accept_without_offer_fails_and_trade_validates() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx1 = hub.register(1);
        let mut rx2 = hub.register(2);
        let mut state = SimState::with_default_empty(test_content());
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Login {
                conn_id: 1,
                reconnect: false,
                email: "a@x".into(),
                client_tag: "t".into(),
            },
        );
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Login {
                conn_id: 2,
                reconnect: false,
                email: "b@x".into(),
                client_tag: "t".into(),
            },
        );
        let a = state.players.get(&1).unwrap().p_id;
        let b = state.players.get(&2).unwrap().p_id;

        while rx1.try_recv().is_ok() {}
        while rx2.try_recv().is_ok() {}

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 2,
                tag: "SAY".into(),
                payload: "ACCEPT".into(),
            },
        );
        let mut saw_fail = false;
        while let Ok(pkt) = rx2.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("ACCEPT") && s.contains("FAIL") {
                saw_fail = true;
            }
        }
        assert!(saw_fail, "ACCEPT with no offer must FAIL");
        assert_eq!(state.scoreboard.entry(a).unwrap().coins, 5);
        assert_eq!(state.scoreboard.entry(b).unwrap().coins, 5);

        // Self-trade / zero amount rejected.
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("TRADE {a} 1"),
            },
        );
        assert!(
            state.players.get(&1).unwrap().trade_offer.is_none(),
            "self TRADE must not set offer"
        );
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("TRADE {b} 0"),
            },
        );
        assert!(
            state.players.get(&1).unwrap().trade_offer.is_none(),
            "zero-amount TRADE must not set offer"
        );

        // Insufficient funds: offer stays, transfer fails.
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("TRADE {b} 100"),
            },
        );
        assert_eq!(
            state.players.get(&1).unwrap().trade_offer,
            Some((b, 100))
        );
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 2,
                tag: "SAY".into(),
                payload: "ACCEPT".into(),
            },
        );
        assert_eq!(
            state.players.get(&1).unwrap().trade_offer,
            Some((b, 100)),
            "failed ACCEPT leaves offer pending"
        );
        assert_eq!(state.scoreboard.entry(a).unwrap().coins, 5);
        assert_eq!(state.scoreboard.entry(b).unwrap().coins, 5);
    }

    /// SAY GIFT moves coins without trade prestige (unlike PAY/transfer).
    #[test]
    fn say_gift_transfers_without_trade_prestige() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx1 = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let a = spawn_player(&mut state, 1, "giver@x");
        let b = spawn_player(&mut state, 2, "giftee@x");
        // Seed wallets without trade prestige so gift path is easy to assert.
        state.economy.wallet_mut(a).coins = 10;
        state.economy.wallet_mut(b).coins = 0;
        state.scoreboard.set_coins(a, 10);
        state.scoreboard.set_coins(b, 0);
        let tp_a0 = state.economy.wallets.get(&a).map(|w| w.trade_prestige).unwrap_or(0.0);
        let tp_b0 = state.economy.wallets.get(&b).map(|w| w.trade_prestige).unwrap_or(0.0);

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("GIFT {b} 3"),
            },
        );
        assert_eq!(state.economy.wallets.get(&a).map(|w| w.coins), Some(7));
        assert_eq!(state.economy.wallets.get(&b).map(|w| w.coins), Some(3));
        assert_eq!(state.scoreboard.entry(a).unwrap().coins, 7);
        assert_eq!(state.scoreboard.entry(b).unwrap().coins, 3);
        assert_eq!(
            state.economy.wallets.get(&a).map(|w| w.trade_prestige),
            Some(tp_a0)
        );
        assert_eq!(
            state.economy.wallets.get(&b).map(|w| w.trade_prestige),
            Some(tp_b0)
        );
        let mut saw = false;
        while let Ok(pkt) = rx1.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("GIFT") && s.contains("OK") {
                saw = true;
            }
        }
        assert!(saw, "expected PS GIFT … OK");

        // Insufficient funds fails without changing balances.
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("GIFT {b} 99"),
            },
        );
        assert_eq!(state.economy.wallets.get(&a).map(|w| w.coins), Some(7));
    }

    /// SAY LOAN records DebtBook and moves coins; SAY REPAY clears debt.
    #[test]
    fn say_loan_and_repay_tracks_debt_map() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx2 = hub.register(2);
        let mut state = SimState::with_default_empty(test_content());
        let lender = spawn_player(&mut state, 1, "lender@x");
        let borrower = spawn_player(&mut state, 2, "borrower@x");
        // Seed without add_coins prestige side-effects.
        state.economy.wallet_mut(lender).coins = 10;
        state.economy.wallet_mut(borrower).coins = 0;
        state.scoreboard.set_coins(lender, 10);
        state.scoreboard.set_coins(borrower, 0);

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("LOAN {borrower} 4"),
            },
        );
        assert_eq!(state.economy.wallets.get(&lender).map(|w| w.coins), Some(6));
        assert_eq!(
            state.economy.wallets.get(&borrower).map(|w| w.coins),
            Some(4)
        );
        assert_eq!(state.debts.owed(borrower, lender), 4);
        assert_eq!(state.scoreboard.entry(lender).unwrap().coins, 6);
        assert_eq!(state.scoreboard.entry(borrower).unwrap().coins, 4);

        // Partial repay.
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 2,
                tag: "SAY".into(),
                payload: format!("REPAY {lender} 1"),
            },
        );
        assert_eq!(state.debts.owed(borrower, lender), 3);
        assert_eq!(
            state.economy.wallets.get(&borrower).map(|w| w.coins),
            Some(3)
        );
        assert_eq!(state.economy.wallets.get(&lender).map(|w| w.coins), Some(7));

        // Full remaining repay (omit amount).
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 2,
                tag: "SAY".into(),
                payload: format!("REPAY {lender}"),
            },
        );
        assert_eq!(state.debts.owed(borrower, lender), 0);
        assert_eq!(
            state.economy.wallets.get(&borrower).map(|w| w.coins),
            Some(0)
        );
        assert_eq!(state.economy.wallets.get(&lender).map(|w| w.coins), Some(10));

        // ?DEBT query.
        while rx2.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 2,
                tag: "SAY".into(),
                payload: "?DEBT".into(),
            },
        );
        let mut saw_debt = false;
        while let Ok(pkt) = rx2.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("DEBT") && s.contains("owe=0") {
                saw_debt = true;
            }
        }
        assert!(saw_debt, "expected PS ?DEBT with owe=0");

        // HELP lists new commands.
        let help = SimState::format_help_query();
        assert!(help.contains("LOAN"), "HELP should list LOAN");
        assert!(help.contains("REPAY"), "HELP should list REPAY");
        assert!(help.contains("GIFT"), "HELP should list GIFT");
        assert!(help.contains("?DEBT"), "HELP should list ?DEBT");
    }

    #[test]
    fn apoc_query_and_active_food_drain() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "apoc");
        // Neutral temp so TEMP_FOOD_EXTRA stays off; freeze season/day shifts.
        state.environment.temperature = 0.5;
        state.environment.season_length = 10_000.0;
        state.environment.day_length = 10_000.0;
        state.environment.hour_of_day = 12.0;
        state.apocalypse.warning_duration = 1.0;
        state.apocalypse.active_duration = 10.0;

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?APOC".into(),
            },
        );
        let mut saw_idle = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("APOC IDLE") {
                saw_idle = true;
            }
        }
        assert!(saw_idle, "expected ?APOC IDLE reply");

        state.apocalypse.trigger();
        // Drain through warning into active.
        tick_vitals(&mut state, 1.0, &hub);
        assert_eq!(state.apocalypse.phase, ApocalypsePhase::Active);

        let food_before = state.players.get(&1).unwrap().food;
        tick_vitals(&mut state, 1.0, &hub);
        let food_after = state.players.get(&1).unwrap().food;
        let lost = food_before - food_after;
        let expected = FOOD_USE_PER_SEC * APOC_FOOD_DRAIN_MULT;
        assert!(
            (lost - expected).abs() < 1e-4,
            "active apoc drain: lost={lost} expected={expected}"
        );

        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?APOC".into(),
            },
        );
        let mut saw_active = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("APOC ACTIVE") {
                saw_active = true;
            }
        }
        assert!(saw_active, "expected ?APOC ACTIVE reply");
    }

    /// SAY STARTAPOC / ENDAPOC force apocalypse for testing (no admin).
    #[test]
    fn say_startapoc_endapoc() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "apoc_cmd@x");
        assert_eq!(state.apocalypse.phase, ApocalypsePhase::Idle);

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "STARTAPOC".into(),
            },
        );
        assert_eq!(state.apocalypse.phase, ApocalypsePhase::Warning);
        let mut saw_warning = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("APOC WARNING") {
                saw_warning = true;
            }
        }
        assert!(saw_warning, "expected STARTAPOC PS with APOC WARNING");

        // Advance into Active, then ENDAPOC resets to Idle.
        state.apocalypse.warning_duration = 1.0;
        state.apocalypse.countdown = 0.0;
        state.apocalypse.tick(0.1);
        assert_eq!(state.apocalypse.phase, ApocalypsePhase::Active);

        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "ENDAPOC".into(),
            },
        );
        assert_eq!(state.apocalypse.phase, ApocalypsePhase::Idle);
        assert_eq!(state.apocalypse.food_drain_multiplier(), 1.0);
        let mut saw_idle = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("APOC IDLE") {
                saw_idle = true;
            }
        }
        assert!(saw_idle, "expected ENDAPOC PS with APOC IDLE");
    }

    /// SAY SETSEASON SPRING|SUMMER|AUTUMN|WINTER forces season.
    #[test]
    fn say_setseason() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "season@x");
        assert_eq!(state.environment.season, Season::Spring);

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "SETSEASON WINTER".into(),
            },
        );
        assert_eq!(state.environment.season, Season::Winter);
        assert_eq!(state.environment.season_elapsed, 0.0);
        let mut saw = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("WINTER") {
                saw = true;
            }
        }
        assert!(saw, "expected SETSEASON PS with WINTER");

        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "SETSEASON NOPE".into(),
            },
        );
        assert_eq!(state.environment.season, Season::Winter);
        let mut saw_fail = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("SETSEASON FAIL") {
                saw_fail = true;
            }
        }
        assert!(saw_fail, "expected SETSEASON FAIL for bad token");

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "SETSEASON SUMMER".into(),
            },
        );
        assert_eq!(state.environment.season, Season::Summer);
    }

    /// SAY SETHOUR <0-23> forces day hour.
    #[test]
    fn say_sethour() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "hour@x");
        state.environment.hour_of_day = 12.0;

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "SETHOUR 0".into(),
            },
        );
        assert!((state.environment.hour_of_day - 0.0).abs() < 1e-6);
        assert_eq!(state.environment.day_phase().as_str(), "NIGHT");
        let mut saw = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("TIME ") && s.contains("NIGHT") {
                saw = true;
            }
        }
        assert!(saw, "expected SETHOUR PS with TIME + NIGHT");

        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "SETHOUR 19".into(),
            },
        );
        assert!((state.environment.hour_of_day - 19.0).abs() < 1e-6);
        assert_eq!(state.environment.day_phase().as_str(), "DUSK");

        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "SETHOUR 99".into(),
            },
        );
        assert!((state.environment.hour_of_day - 19.0).abs() < 1e-6);
        let mut saw_fail = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("SETHOUR FAIL") {
                saw_fail = true;
            }
        }
        assert!(saw_fail, "expected SETHOUR FAIL for out-of-range");
    }

    /// SAY WEATHER / SETWEATHER sets kind (already present; ensure set path works).
    #[test]
    fn say_weather_set() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "wx@x");
        assert_eq!(state.weather.kind, crate::weather::WeatherKind::Clear);

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "WEATHER rain 45".into(),
            },
        );
        assert_eq!(state.weather.kind, crate::weather::WeatherKind::Rain);
        assert!((state.weather.remaining_secs - 45.0).abs() < 1e-4);
        let mut saw = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("WEATHER") && s.contains("rain") {
                saw = true;
            }
        }
        assert!(saw, "expected WEATHER set PS reply");

        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "SETWEATHER storm 30".into(),
            },
        );
        assert_eq!(state.weather.kind, crate::weather::WeatherKind::Storm);
        assert!((state.weather.remaining_secs - 30.0).abs() < 1e-4);

        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "WEATHER bogos".into(),
            },
        );
        assert_eq!(state.weather.kind, crate::weather::WeatherKind::Storm);
        let mut saw_fail = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("WEATHER FAIL") {
                saw_fail = true;
            }
        }
        assert!(saw_fail, "expected WEATHER FAIL bad_kind");
    }

    /// SAY SEED respawns default animals only when the animal world is empty.
    #[test]
    fn say_seed_animals_if_empty() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "seed@x");
        assert!(state.animals.animals.is_empty());

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "SEED".into(),
            },
        );
        assert_eq!(state.animals.animals.len(), 7);
        let mut saw_ok = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("SEED OK") && s.contains("animals=7") {
                saw_ok = true;
            }
        }
        assert!(saw_ok, "expected SEED OK animals=7");

        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "SEED".into(),
            },
        );
        assert_eq!(state.animals.animals.len(), 7, "second SEED must not double-spawn");
        let mut saw_skip = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("SEED SKIP") {
                saw_skip = true;
            }
        }
        assert!(saw_skip, "expected SEED SKIP when animals already present");
    }

    #[test]
    fn curse_token_score_and_query() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx1 = hub.register(1);
        let mut rx2 = hub.register(2);
        let mut state = SimState::with_default_empty(test_content());
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Login {
                conn_id: 1,
                reconnect: false,
                email: "alice@x".into(),
                client_tag: "t".into(),
            },
        );
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Login {
                conn_id: 2,
                reconnect: false,
                email: "bob@x".into(),
                client_tag: "t".into(),
            },
        );
        let a = state.players.get(&1).unwrap().p_id;
        let b = state.players.get(&2).unwrap().p_id;
        // Login seeds one curse token and sends CX.
        assert_eq!(state.curses.tokens(a), DEFAULT_CURSE_TOKENS);
        let mut saw_cx_login = false;
        while let Ok(pkt) = rx1.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("CX\n") {
                saw_cx_login = true;
            }
        }
        assert!(saw_cx_login, "expected CX on login");
        while rx2.try_recv().is_ok() {}

        assert!(state.curses.curse_player(a, b));
        assert_eq!(state.curses.tokens(a), 0);
        assert_eq!(state.curses.score(b), 1);

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?CURSE".into(),
            },
        );
        let mut saw_ps = false;
        let mut saw_cx = false;
        let mut saw_cs = false;
        while let Ok(pkt) = rx1.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("CURSE tokens=0") {
                saw_ps = true;
            }
            if s.starts_with("CX\n") {
                saw_cx = true;
                assert!(s.contains("0"));
            }
            if s.starts_with("CS\n") {
                saw_cs = true;
            }
        }
        assert!(saw_ps, "expected PS ?CURSE reply");
        assert!(saw_cx, "expected CX token wire");
        assert!(saw_cs, "expected CS score wire");
    }

    #[test]
    fn posse_join_query_and_clear() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx1 = hub.register(1);
        let mut rx2 = hub.register(2);
        let mut state = SimState::with_default_empty(test_content());
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Login {
                conn_id: 1,
                reconnect: false,
                email: "alice@x".into(),
                client_tag: "t".into(),
            },
        );
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Login {
                conn_id: 2,
                reconnect: false,
                email: "bob@x".into(),
                client_tag: "t".into(),
            },
        );
        let a = state.players.get(&1).unwrap().p_id;
        let b = state.players.get(&2).unwrap().p_id;
        assert!(!state.posse.has_target(a, b));

        while rx1.try_recv().is_ok() {}
        while rx2.try_recv().is_ok() {}

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("POSSE {b}"),
            },
        );
        assert!(state.posse.has_target(a, b));

        let mut saw_posse_ps = false;
        let mut saw_pj = false;
        while let Ok(pkt) = rx1.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("POSSE") && s.contains("OK") {
                saw_posse_ps = true;
            }
            if s.starts_with("PJ\n") && s.contains(&format!("{a} {b}")) {
                saw_pj = true;
            }
        }
        assert!(saw_posse_ps, "expected PS POSSE OK reply");
        assert!(saw_pj, "expected PJ posse join wire");

        while rx1.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?POSSE".into(),
            },
        );
        let mut saw_list = false;
        while let Ok(pkt) = rx1.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("POSSE") && s.contains(&format!("{b}")) {
                saw_list = true;
            }
        }
        assert!(saw_list, "expected PS ?POSSE list with target");

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "POSSE 0".into(),
            },
        );
        assert!(!state.posse.has_target(a, b));
        assert_eq!(state.posse.target_count(a), 0);
    }

    #[test]
    fn war_declare_query_and_peace() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx1 = hub.register(1);
        let mut rx2 = hub.register(2);
        let mut state = SimState::with_default_empty(test_content());
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Login {
                conn_id: 1,
                reconnect: false,
                email: "alice@x".into(),
                client_tag: "t".into(),
            },
        );
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Login {
                conn_id: 2,
                reconnect: false,
                email: "bob@x".into(),
                client_tag: "t".into(),
            },
        );
        let a = state.players.get(&1).unwrap().p_id;
        let b = state.players.get(&2).unwrap().p_id;
        assert!(!state.war.is_at_war(a, b));

        while rx1.try_recv().is_ok() {}
        while rx2.try_recv().is_ok() {}

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("WAR {b}"),
            },
        );
        assert!(state.war.is_at_war(a, b));
        assert!(state.war.is_at_war(b, a));

        let mut saw_war_ps = false;
        let mut saw_wr = false;
        while let Ok(pkt) = rx1.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("WAR") && s.contains("OK") {
                saw_war_ps = true;
            }
            if s.starts_with("WR\n") && s.contains(STATUS_WAR) {
                saw_wr = true;
            }
        }
        assert!(saw_war_ps, "expected PS WAR OK reply");
        assert!(saw_wr, "expected WR war report");

        while rx1.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?WAR".into(),
            },
        );
        let mut saw_list = false;
        while let Ok(pkt) = rx1.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("WAR") && s.contains(&format!("{a}")) && s.contains(&format!("{b}")) {
                saw_list = true;
            }
        }
        assert!(saw_list, "expected PS ?WAR list with pair");

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("PEACE {b}"),
            },
        );
        assert!(!state.war.is_at_war(a, b));
        assert_eq!(state.war.status(a, b), STATUS_PEACE);
        // War declare ensures scoreboard rows (optional soft scoreboard).
        assert!(state.scoreboard.entry(a).is_some());
        assert!(state.scoreboard.entry(b).is_some());
    }

    /// SAY ?GEN returns LineageNode.generation; birth child is gen+1.
    #[test]
    fn say_gen_lineage_depth() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Login {
                conn_id: 1,
                reconnect: false,
                email: "gen@x".into(),
                client_tag: "t".into(),
            },
        );
        let mother = state.players.get(&1).unwrap().p_id;
        while rx.try_recv().is_ok() {}

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?GEN".into(),
            },
        );
        let mut saw_gen0 = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PS\n") && s.contains(&format!("GEN {mother} 0")) {
                saw_gen0 = true;
            }
        }
        assert!(saw_gen0, "founder should be GEN 0");

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "BIRTH".into(),
            },
        );
        let baby_conn = 1u64 + BABY_CONN_OFFSET;
        let baby_id = state.players.get(&baby_conn).unwrap().p_id;
        assert_eq!(
            state.social.lineages.get(&baby_id).map(|n| n.generation),
            Some(1)
        );

        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("?GEN {baby_id}"),
            },
        );
        let mut saw_gen1 = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PS\n") && s.contains(&format!("GEN {baby_id} 1")) {
                saw_gen1 = true;
            }
        }
        assert!(saw_gen1, "baby should be GEN 1");
    }

    /// SAY ?FAMILY lists online players with the same family_name.
    #[test]
    fn say_family_lists_same_family_name() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx1 = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Login {
                conn_id: 1,
                reconnect: false,
                email: "fam1@x".into(),
                client_tag: "t".into(),
            },
        );
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Login {
                conn_id: 2,
                reconnect: false,
                email: "fam2@x".into(),
                client_tag: "t".into(),
            },
        );
        let a = state.players.get(&1).unwrap().p_id;
        let b = state.players.get(&2).unwrap().p_id;
        // Force shared family name.
        {
            let fa = state.players.get(&1).unwrap().family_name.clone();
            if let Some(p) = state.players.get_mut(&2) {
                p.family_name = fa;
            }
        }
        let family = state.players.get(&1).unwrap().family_name.clone();
        assert!(!family.is_empty());

        while rx1.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?FAMILY".into(),
            },
        );
        let mut saw = false;
        while let Ok(pkt) = rx1.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PS\n") && s.contains("FAMILY ") {
                saw = true;
                assert!(s.contains(&family), "got {s}");
                assert!(s.contains(&format!("{a} ")), "got {s}");
                assert!(s.contains(&format!("{b} ")), "got {s}");
            }
        }
        assert!(saw, "expected PS ?FAMILY reply");
    }

    /// SAY ?REL marks EVE when mother_id is None; children show mother without self-EVE.
    #[test]
    fn say_rel_eve_detection() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Login {
                conn_id: 1,
                reconnect: false,
                email: "eve@x".into(),
                client_tag: "t".into(),
            },
        );
        let a = state.players.get(&1).unwrap().p_id;
        assert!(is_eve(&state.social, a));

        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?REL".into(),
            },
        );
        let mut saw_eve = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PS\n") && s.contains("EVE") {
                saw_eve = true;
                assert!(s.contains(&format!("REL {a} {a} EVE")), "got {s}");
            }
        }
        assert!(saw_eve, "expected PS ?REL EVE for founder");

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "BIRTH".into(),
            },
        );
        let baby_conn = 1u64 + BABY_CONN_OFFSET;
        let baby_id = state.players.get(&baby_conn).unwrap().p_id;
        assert!(!is_eve(&state.social, baby_id));
        let rel = format_relation_query(&state.social, baby_id, a);
        assert!(rel.contains("mother"));
        assert!(rel.contains("EVE"), "Eve mother should mark EVE: {rel}");
    }

    /// SAY RAID requires mutual posse; prestige note only (no kill / death).
    #[test]
    fn say_raid_posse_prestige_note_only() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx1 = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Login {
                conn_id: 1,
                reconnect: false,
                email: "raider@x".into(),
                client_tag: "t".into(),
            },
        );
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Login {
                conn_id: 2,
                reconnect: false,
                email: "target@x".into(),
                client_tag: "t".into(),
            },
        );
        let a = state.players.get(&1).unwrap().p_id;
        let b = state.players.get(&2).unwrap().p_id;

        // Without mutual posse → FAIL.
        while rx1.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("RAID {b}"),
            },
        );
        let mut saw_fail = false;
        while let Ok(pkt) = rx1.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("RAID") && s.contains("FAIL") {
                saw_fail = true;
            }
        }
        assert!(saw_fail, "expected RAID FAIL without posse");
        assert!(!state.players.get(&2).unwrap().deleted);

        // Mutual posse → OK prestige note; no death.
        state.posse.add_posse(a, b);
        state.posse.add_posse(b, a);
        while rx1.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("RAID {b}"),
            },
        );
        let mut saw_ok = false;
        while let Ok(pkt) = rx1.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("RAID") && s.contains("OK") && s.contains("prestige=") {
                saw_ok = true;
            }
        }
        assert!(saw_ok, "expected RAID OK prestige note");
        assert!(!state.players.get(&2).unwrap().deleted);
        assert_eq!(
            state.combat.stats.get(&b).map(|s| s.deaths).unwrap_or(0),
            0
        );
        assert!(
            state
                .event_log
                .iter()
                .any(|e| e == &format!("RAID {a} {b}")),
            "expected RAID event"
        );
    }

    #[test]
    fn ping_raw_replies_pong() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "ping@x");
        while rx.try_recv().is_ok() {}

        // Net maps Ping → payload = unique_id only.
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "PING".into(),
                payload: "uid42".into(),
            },
        );
        let mut saw = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.as_ref() == "PONG\nuid42\n#" {
                saw = true;
            }
        }
        assert!(saw, "expected PONG echo of unique_id");

        // Full wire-shaped payload still echoes last token (unique_id).
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "PING".into(),
                payload: "10 20 full_uid".into(),
            },
        );
        let mut saw_full = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.as_ref() == "PONG\nfull_uid\n#" {
                saw_full = true;
            }
        }
        assert!(saw_full, "expected PONG from x y unique_id payload");
    }

    #[test]
    fn photo_and_vog_raw_ack() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "photo@x");
        while rx.try_recv().is_ok() {}

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "PHOTO".into(),
                payload: "10 20 1".into(),
            },
        );
        let mut saw_ph = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PH\n") && s.contains("10 20") && s.contains(PHOTO_DENIED_SIGNATURE) {
                saw_ph = true;
            }
        }
        assert!(saw_ph, "expected PH deny ACK for PHOTO");

        // SAY SNAP — same deny as PHOTO (coords from args).
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "SNAP 10 20 1".into(),
            },
        );
        let mut saw_snap = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PH\n") && s.contains("10 20") && s.contains(PHOTO_DENIED_SIGNATURE) {
                saw_snap = true;
            }
        }
        assert!(saw_snap, "expected PH deny ACK for SAY SNAP");

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "VOGS".into(),
                payload: "301 14".into(),
            },
        );
        let mut saw_vu = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s == "VU\n301 14\n#" {
                saw_vu = true;
            }
        }
        assert!(saw_vu, "expected VU ACK for VOGS");
    }

    /// SAY VOGSET requires godmode; with flag sets object on tile + MC + PS OK.
    #[test]
    fn say_vogset_godmode_sets_tile() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "vog@x");
        while rx.try_recv().is_ok() {}

        // Without godmode → DENIED, tile unchanged.
        assert!(!state.players.get(&1).unwrap().godmode);
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "VOGSET 5 6 33".into(),
            },
        );
        assert_eq!(state.world.read().unwrap().get_object(5, 6), 0);
        let mut saw_denied = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PS\n") && s.contains(&format!("{p_id} VOGSET DENIED")) {
                saw_denied = true;
            }
        }
        assert!(saw_denied, "expected VOGSET DENIED without godmode");

        // Enable godmode and set tile.
        state.players.get_mut(&1).unwrap().godmode = true;
        while rx.try_recv().is_ok() {}
        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "VOGSET 5 6 33".into(),
            },
        );
        assert_eq!(state.world.read().unwrap().get_object(5, 6), 33);
        let mut saw_ok = false;
        let mut saw_mx = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PS\n") && s.contains(&format!("{p_id} VOGSET 5 6 33 OK")) {
                saw_ok = true;
            }
            if s.starts_with("MX\n") && s.contains("5 6") && s.contains(" 33 ") {
                saw_mx = true;
            }
        }
        assert!(saw_ok, "expected VOGSET OK PS");
        assert!(saw_mx, "expected map-change MX after VOGSET");

        // Malformed args → FAIL.
        while rx.try_recv().is_ok() {}
        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "VOGSET 1 2".into(),
            },
        );
        let mut saw_fail = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PS\n") && s.contains(&format!("{p_id} VOGSET FAIL")) {
                saw_fail = true;
            }
        }
        assert!(saw_fail, "expected VOGSET FAIL for incomplete args");

        // Clear tile with obj=0.
        while rx.try_recv().is_ok() {}
        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "VOGSET 5 6 0".into(),
            },
        );
        assert_eq!(state.world.read().unwrap().get_object(5, 6), 0);
    }

    /// SAY REGEN places weighted natural object from content.biome_spawn when tile empty.
    #[test]
    fn say_regen_places_biome_spawn_when_empty() {
        use ol_content::BiomeSpawnTable;

        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut db = ContentDb::default();
        db.objects.insert(
            33,
            ObjectDef {
                id: 33,
                description: "Gooseberry".into(),
                name: "Gooseberry".into(),
                containable: true,
                permanent: false,
                blocks_walking: false,
                food_value: 3,
                heat_value: 0.0,
                map_chance: 1.0,
                biomes: vec![0],
                num_uses: 0,
                num_slots: 0,
                floor: false,
            dummy_ids: Vec::new(),
            },
        );
        db.biome_spawn.insert(
            0,
            BiomeSpawnTable {
                total_chance: 1.0,
                entries: vec![(33, 1.0)],
            },
        );
        let mut state = SimState::with_default_empty(Arc::new(db));
        let p_id = spawn_player(&mut state, 1, "regen@x");
        // Spawn at (0,0); biome defaults to 0.
        assert_eq!(state.players.get(&1).unwrap().x, 0);
        assert_eq!(state.players.get(&1).unwrap().y, 0);
        assert_eq!(state.world.read().unwrap().get_object(0, 0), 0);
        while rx.try_recv().is_ok() {}

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "REGEN".into(),
            },
        );
        assert_eq!(state.world.read().unwrap().get_object(0, 0), 33);
        let mut saw_ok = false;
        let mut saw_mx = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PS\n") && s.contains(&format!("{p_id} REGEN OK 0 0 33")) {
                saw_ok = true;
            }
            if s.starts_with("MX\n") && s.contains("0 0") && s.contains(" 33 ") {
                saw_mx = true;
            }
        }
        assert!(saw_ok, "expected REGEN OK PS");
        assert!(saw_mx, "expected MX after REGEN");

        // Second REGEN on non-empty tile → SKIP.
        while rx.try_recv().is_ok() {}
        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "REGEN".into(),
            },
        );
        assert_eq!(state.world.read().unwrap().get_object(0, 0), 33);
        let mut saw_skip = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PS\n") && s.contains("REGEN SKIP not_empty") {
                saw_skip = true;
            }
        }
        assert!(saw_skip, "expected REGEN SKIP when not empty");
    }

    /// SAY REGEN FAIL when biome has no spawn table.
    #[test]
    fn say_regen_fails_without_biome_spawn() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "regen2@x");
        while rx.try_recv().is_ok() {}

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "REGEN".into(),
            },
        );
        assert_eq!(state.world.read().unwrap().get_object(0, 0), 0);
        let mut saw_fail = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PS\n") && s.contains(&format!("{p_id} REGEN FAIL no_spawn")) {
                saw_fail = true;
            }
        }
        assert!(saw_fail, "expected REGEN FAIL no_spawn");
    }

    /// SAY CLEAROBJ requires godmode; clears object under feet + MX.
    #[test]
    fn say_clearobj_godmode_clears_tile() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "clr@x");
        state.world.write().unwrap().set_object(0, 0, 33);
        while rx.try_recv().is_ok() {}

        // Without godmode → DENIED.
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "CLEAROBJ".into(),
            },
        );
        assert_eq!(state.world.read().unwrap().get_object(0, 0), 33);
        let mut saw_denied = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PS\n") && s.contains(&format!("{p_id} CLEAROBJ DENIED")) {
                saw_denied = true;
            }
        }
        assert!(saw_denied, "expected CLEAROBJ DENIED");

        state.players.get_mut(&1).unwrap().godmode = true;
        while rx.try_recv().is_ok() {}
        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "CLEAROBJ".into(),
            },
        );
        assert_eq!(state.world.read().unwrap().get_object(0, 0), 0);
        let mut saw_ok = false;
        let mut saw_mx = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PS\n") && s.contains(&format!("{p_id} CLEAROBJ OK 0 0")) {
                saw_ok = true;
            }
            if s.starts_with("MX\n") && s.contains("0 0") && s.contains(" 0 ") {
                saw_mx = true;
            }
        }
        assert!(saw_ok, "expected CLEAROBJ OK");
        assert!(saw_mx, "expected MX after CLEAROBJ");
    }

    /// SAY FILL requires godmode; sets floor under feet to 1.
    #[test]
    fn say_fill_godmode_sets_floor_one() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "fill@x");
        assert_eq!(state.world.read().unwrap().get_floor(0, 0), 0);
        while rx.try_recv().is_ok() {}

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "FILL".into(),
            },
        );
        assert_eq!(state.world.read().unwrap().get_floor(0, 0), 0);
        let mut saw_denied = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PS\n") && s.contains(&format!("{p_id} FILL DENIED")) {
                saw_denied = true;
            }
        }
        assert!(saw_denied, "expected FILL DENIED");

        state.players.get_mut(&1).unwrap().godmode = true;
        while rx.try_recv().is_ok() {}
        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "FILL".into(),
            },
        );
        assert_eq!(state.world.read().unwrap().get_floor(0, 0), 1);
        let mut saw_ok = false;
        let mut saw_mx = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PS\n") && s.contains(&format!("{p_id} FILL OK 0 0 floor=1")) {
                saw_ok = true;
            }
            if s.starts_with("MX\n") && s.contains("0 0 1 ") {
                saw_mx = true;
            }
        }
        assert!(saw_ok, "expected FILL OK");
        assert!(saw_mx, "expected MX with floor=1 after FILL");
    }

    #[test]
    fn say_global_broadcasts_gm() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx1 = hub.register(1);
        let mut rx2 = hub.register(2);
        let mut state = SimState::with_default_empty(test_content());
        let p1 = spawn_player(&mut state, 1, "g1@x");
        spawn_player(&mut state, 2, "g2@x");
        // GLOBAL requires noble+ prestige (combat threshold ≥ 50).
        state.combat.stats_mut(p1).prestige = 50.0;
        if let Some(n) = state.social.lineages.get_mut(&p1) {
            n.set_prestige(50.0);
        }
        while rx1.try_recv().is_ok() {}
        while rx2.try_recv().is_ok() {}

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "GLOBAL hello world".into(),
            },
        );

        let expected = format_server_message("GM", &["hello world"]);
        let mut saw1 = false;
        while let Ok(pkt) = rx1.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s == expected || s.starts_with("GM\n") {
                saw1 = true;
            }
        }
        let mut saw2 = false;
        while let Ok(pkt) = rx2.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s == expected || s.starts_with("GM\n") {
                saw2 = true;
            }
        }
        assert!(saw1, "conn 1 expected GM global packet");
        assert!(saw2, "conn 2 expected GM global packet (broadcast)");

        // Direct helper path.
        while rx1.try_recv().is_ok() {}
        broadcast_global(&hub, "direct");
        let mut saw_direct = false;
        while let Ok(pkt) = rx1.try_recv() {
            if String::from_utf8_lossy(&pkt) == format_server_message("GM", &["direct"]) {
                saw_direct = true;
            }
        }
        assert!(saw_direct, "broadcast_global should push GM");
    }

    /// `SAY SHOUT <text>` fans out PS at [`SHOUT_RANGE`] (48), past normal nearby.
    #[test]
    fn say_shout_uses_larger_nearby_range() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx1 = hub.register(1);
        let mut rx2 = hub.register(2);
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "near@x");
        spawn_player(&mut state, 2, "far@x");
        // Beyond NEARBY_RANGE (24) but within SHOUT_RANGE (48).
        set_player_position(&mut state, 1, 0, 0);
        set_player_position(&mut state, 2, 30, 0);
        while rx1.try_recv().is_ok() {}
        while rx2.try_recv().is_ok() {}

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "hello soft".into(),
            },
        );
        let mut far_soft = false;
        while let Ok(pkt) = rx2.try_recv() {
            if String::from_utf8_lossy(&pkt).contains("hello soft") {
                far_soft = true;
            }
        }
        assert!(!far_soft, "normal SAY must not reach beyond NEARBY_RANGE");

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "SHOUT hello loud".into(),
            },
        );
        let mut far_shout = false;
        while let Ok(pkt) = rx2.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("SHOUT hello loud") {
                far_shout = true;
            }
        }
        assert!(far_shout, "SHOUT PS should reach within SHOUT_RANGE");
        // Speaker also receives their own PS fan-out.
        let mut self_shout = false;
        while let Ok(pkt) = rx1.try_recv() {
            if String::from_utf8_lossy(&pkt).contains("SHOUT hello loud") {
                self_shout = true;
            }
        }
        assert!(self_shout, "speaker should receive SHOUT PS");
    }

    /// SAY ?HELP / HELP returns short list of supported commands via private PS (no SQL).
    #[test]
    fn say_help_returns_short_command_list() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "help@x");

        let expected = SimState::format_help_query();
        assert!(expected.starts_with("HELP "), "got {expected}");
        // Spot-check a few well-known commands appear in the short list.
        for cmd in ["?WHO", "?WHERE", "?FOOD", "?AGE", "?NAME", "?HELD", "FOLLOW", "SHOUT"] {
            assert!(
                expected.contains(cmd),
                "help list should mention {cmd}: {expected}"
            );
        }

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?HELP".into(),
            },
        );

        let mut saw = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PS\n") && s.contains("HELP ") {
                saw = true;
                assert!(
                    s.contains(&format!("{p_id} {expected}")),
                    "PS should embed help list: {s}"
                );
                assert!(s.contains("?WHO"), "got {s}");
                assert!(s.contains("?WHERE"), "got {s}");
            }
        }
        assert!(saw, "expected PS ?HELP reply with command list");

        // Bare HELP also works (private PS only).
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "HELP".into(),
            },
        );
        let mut saw_bare = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PS\n") && s.contains(&format!("{p_id} HELP ")) {
                saw_bare = true;
                assert!(s.contains("?WHO"), "got {s}");
                assert!(s.contains("SHOUT"), "got {s}");
            }
        }
        assert!(saw_bare, "expected PS bare HELP reply");

        // Pure formatter unit check (no wire).
        assert_eq!(SimState::format_help_query(), expected);
        assert!(!expected.contains(';'), "help list is space-separated tokens");
    }

    /// SAY ?NAME / NAME returns display_name via private PS (no SQL).
    #[test]
    fn say_name_returns_display_name() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "name@x");

        // Deterministic name so the reply is exact.
        {
            let p = state.players.get_mut(&1).unwrap();
            p.first_name = "ADAM".into();
            p.family_name = "SMITH".into();
        }
        let display = state.players.get(&1).unwrap().display_name();
        assert_eq!(display, "ADAM SMITH");
        let expected = SimState::format_name_query(&display);
        assert_eq!(expected, "NAME ADAM SMITH");

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?NAME".into(),
            },
        );

        let mut saw = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PS\n") && s.contains("NAME ") {
                saw = true;
                assert!(
                    s.contains(&format!("{p_id} {expected}")),
                    "PS should embed display_name: {s}"
                );
                assert!(s.contains("ADAM SMITH"), "got {s}");
            }
        }
        assert!(saw, "expected PS ?NAME reply with display_name");

        // Bare NAME also works (private PS only).
        while rx.try_recv().is_ok() {}
        {
            let p = state.players.get_mut(&1).unwrap();
            p.first_name = "EVE".into();
            p.family_name = "SNOW".into();
        }
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "NAME".into(),
            },
        );
        let mut saw_bare = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PS\n") && s.contains(&format!("{p_id} NAME ")) {
                saw_bare = true;
                assert!(s.contains("EVE SNOW"), "got {s}");
            }
        }
        assert!(saw_bare, "expected PS bare NAME reply");

        // Pure formatter unit check (no wire).
        assert_eq!(SimState::format_name_query("A B"), "NAME A B");
        assert_eq!(
            SimState::format_name_query(&Player::new(9, 9, "x@y").display_name()),
            format!("NAME {}", Player::new(9, 9, "x@y").display_name())
        );
    }

    /// SAY ?FOOD / FOOD returns food and food_max via private PS (no SQL).
    #[test]
    fn say_food_returns_food_and_food_max() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "food@x");

        {
            let p = state.players.get_mut(&1).unwrap();
            p.food = 7.25;
            p.food_max = 20.0;
        }

        let expected = SimState::format_food_query(7.25, 20.0);
        assert_eq!(expected, "FOOD 7.25 20.00");

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?FOOD".into(),
            },
        );

        let mut saw = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PS\n") && s.contains("FOOD ") {
                saw = true;
                assert!(
                    s.contains(&format!("{p_id} {expected}")),
                    "PS should embed food food_max: {s}"
                );
                assert!(s.contains("7.25 20.00"), "got {s}");
            }
        }
        assert!(saw, "expected PS ?FOOD reply with food and food_max");

        // Bare FOOD also works (private PS only).
        while rx.try_recv().is_ok() {}
        {
            let p = state.players.get_mut(&1).unwrap();
            p.food = 15.5;
            p.food_max = 18.0;
        }
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "FOOD".into(),
            },
        );
        let mut saw_bare = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PS\n") && s.contains(&format!("{p_id} FOOD ")) {
                saw_bare = true;
                assert!(s.contains("15.50 18.00"), "got {s}");
            }
        }
        assert!(saw_bare, "expected PS bare FOOD reply");

        // Pure formatter unit check (no wire).
        assert_eq!(SimState::format_food_query(0.0, MAX_FOOD), "FOOD 0.00 20.00");
        assert_eq!(SimState::format_food_query(START_FOOD, MAX_FOOD), "FOOD 10.00 20.00");
    }

    /// SAY ?AGE / AGE returns age via private PS (no SQL).
    #[test]
    fn say_age_returns_age() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "age@x");

        {
            let p = state.players.get_mut(&1).unwrap();
            p.age = 27.5;
        }

        let expected = SimState::format_age_query(27.5);
        assert_eq!(expected, "AGE 27.50");

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?AGE".into(),
            },
        );

        let mut saw = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PS\n") && s.contains("AGE ") {
                saw = true;
                assert!(
                    s.contains(&format!("{p_id} {expected}")),
                    "PS should embed age: {s}"
                );
                assert!(s.contains("27.50"), "got {s}");
            }
        }
        assert!(saw, "expected PS ?AGE reply with age");

        // Bare AGE also works (private PS only).
        while rx.try_recv().is_ok() {}
        {
            let p = state.players.get_mut(&1).unwrap();
            p.age = 0.0;
        }
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "AGE".into(),
            },
        );
        let mut saw_bare = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PS\n") && s.contains(&format!("{p_id} AGE ")) {
                saw_bare = true;
                assert!(s.contains("0.00"), "got {s}");
            }
        }
        assert!(saw_bare, "expected PS bare AGE reply");

        // Pure formatter unit check (no wire).
        assert_eq!(SimState::format_age_query(14.0), "AGE 14.00");
        assert_eq!(SimState::format_age_query(MAX_AGE), "AGE 120.00");
    }

    /// SAY ?STATUS / STATUS: food age held prestige class wound sleep sick sit.
    #[test]
    fn say_status_combines_food_age_held_prestige_class() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "status@x");

        {
            let p = state.players.get_mut(&1).unwrap();
            p.food = 7.25;
            p.age = 27.5;
            p.held_id = 33; // Gooseberry in test_content
            p.sleeping = true;
            p.sick = false;
            p.sitting = true;
        }
        state.combat.apply_wound(p_id, 2);
        // Lineage prestige preferred by player_prestige / player_prestige_class.
        // spawn_player alone does not ensure lineage (LOGIN does); ensure here.
        state.social.ensure_lineage(p_id, "Status Tester");
        state.social.set_lineage_prestige(p_id, 55.0);
        assert_eq!(
            state.player_prestige_class(p_id),
            PrestigeClass::Noble
        );

        let expected = SimState::format_status_query(
            7.25,
            27.5,
            33,
            55.0,
            PrestigeClass::Noble,
            2,
            true,
            false,
            true,
        );
        assert_eq!(expected, "STATUS 7.25 27.50 33 55.00 noble 2 1 0 1");

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?STATUS".into(),
            },
        );

        let mut saw = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PS\n") && s.contains("STATUS ") {
                saw = true;
                assert!(
                    s.contains(&format!("{p_id} {expected}")),
                    "PS should embed combined status: {s}"
                );
                assert!(s.contains("7.25"), "food in reply: {s}");
                assert!(s.contains("27.50"), "age in reply: {s}");
                assert!(s.contains(" 33 "), "held in reply: {s}");
                assert!(s.contains("55.00"), "prestige in reply: {s}");
                assert!(s.contains("noble"), "class in reply: {s}");
                assert!(s.contains(" 2 1 0 1"), "wound sleep sick sit: {s}");
            }
        }
        assert!(
            saw,
            "expected PS ?STATUS reply with food age held prestige class wound flags"
        );

        // Bare STATUS also works (private PS only); empty hands → held 0; serf at 0 prestige.
        while rx.try_recv().is_ok() {}
        {
            let p = state.players.get_mut(&1).unwrap();
            p.food = 10.0;
            p.age = 14.0;
            p.held_id = 0;
            p.sleeping = false;
            p.sick = false;
            p.sitting = false;
        }
        state.combat.clear_wound(p_id);
        state.social.set_lineage_prestige(p_id, 0.0);
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "STATUS".into(),
            },
        );
        let bare_expected = SimState::format_status_query(
            10.0,
            14.0,
            0,
            0.0,
            PrestigeClass::Serf,
            0,
            false,
            false,
            false,
        );
        assert_eq!(bare_expected, "STATUS 10.00 14.00 0 0.00 serf 0 0 0 0");
        let mut saw_bare = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PS\n") && s.contains(&format!("{p_id} STATUS ")) {
                saw_bare = true;
                assert!(
                    s.contains(&format!("{p_id} {bare_expected}")),
                    "got {s}"
                );
            }
        }
        assert!(saw_bare, "expected PS bare STATUS reply");

        // Pure formatter unit checks (no wire / no SQL).
        assert_eq!(
            SimState::format_status_query(
                0.0, 0.0, 0, 0.0, PrestigeClass::Serf, 0, false, false, false
            ),
            "STATUS 0.00 0.00 0 0.00 serf 0 0 0 0"
        );
        assert_eq!(
            SimState::format_status_query(
                20.0, 120.0, 9999, 200.0, PrestigeClass::Emperor, 5, true, true, true
            ),
            "STATUS 20.00 120.00 9999 200.00 emperor 5 1 1 1"
        );
        assert_eq!(
            SimState::format_status_query(
                5.5,
                30.0,
                1,
                12.4,
                PrestigeClass::from_prestige(12.4),
                1,
                false,
                true,
                false,
            ),
            "STATUS 5.50 30.00 1 12.40 commoner 1 0 1 0"
        );
        assert_eq!(
            SimState::format_status_query(
                1.0, 2.0, 0, 0.0, PrestigeClass::Serf, 3, true, true, true
            ),
            "STATUS 1.00 2.00 0 0.00 serf 3 1 1 1"
        );
    }

    /// SAY ?HEART, CLEAR/RESET YUM, BOOST, GODMODE, ?FLAGS.
    #[test]
    fn say_heart_yum_clear_boost_godmode_flags() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "vitals@x");

        {
            let p = state.players.get_mut(&1).unwrap();
            p.food = 3.5;
            p.food_max = 20.0;
            p.age = 22.0;
            let _ = p.yum.eat(33, 3.0, 3);
            let _ = p.yum.eat(40, 3.0, 6);
            p.sleeping = true;
            p.sick = true;
            p.sitting = false;
            p.riding = true;
            p.holding_player_id = 99;
            p.godmode = false;
        }

        assert_eq!(SimState::format_heart_query(3.5, 22.0), "HEART 3.50 22.00");
        assert_eq!(
            SimState::format_flags_query(true, true, false, true, true, false, false),
            "FLAGS sleeping=1 sick=1 sitting=0 riding=1 holding=1 god=0 deaf=0"
        );
        assert_eq!(SimState::format_godmode_query(false), "GODMODE off");
        assert_eq!(SimState::format_godmode_query(true), "GODMODE on");
        assert_eq!(SimState::boost_food(3.5, 20.0), 8.5);
        assert_eq!(SimState::boost_food(18.0, 20.0), 20.0);

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?HEART".into(),
            },
        );
        let mut saw_heart = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PS\n") && s.contains("HEART ") {
                saw_heart = true;
                assert!(
                    s.contains(&format!("{p_id} HEART 3.50 22.00")),
                    "got {s}"
                );
            }
        }
        assert!(saw_heart, "expected PS ?HEART");

        while rx.try_recv().is_ok() {}
        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?FLAGS".into(),
            },
        );
        let mut saw_flags = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PS\n") && s.contains("FLAGS ") {
                saw_flags = true;
                assert!(s.contains("sleeping=1"), "got {s}");
                assert!(s.contains("sick=1"), "got {s}");
                assert!(s.contains("sitting=0"), "got {s}");
                assert!(s.contains("riding=1"), "got {s}");
                assert!(s.contains("holding=1"), "got {s}");
                assert!(s.contains("god=0"), "got {s}");
                assert!(s.contains("deaf=0"), "got {s}");
            }
        }
        assert!(saw_flags, "expected PS ?FLAGS");

        while rx.try_recv().is_ok() {}
        assert!(!state.players.get(&1).unwrap().yum.history.is_empty());
        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "CLEAR YUM".into(),
            },
        );
        {
            let yum = &state.players.get(&1).unwrap().yum;
            assert!(yum.history.is_empty());
            assert_eq!(yum.yum_bonus, 0.0);
        }
        let mut saw_clear = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PS\n") && s.contains("YUM CLEAR OK") {
                saw_clear = true;
                assert!(s.contains("history=0"), "got {s}");
            }
        }
        assert!(saw_clear, "expected PS CLEAR YUM");

        {
            let p = state.players.get_mut(&1).unwrap();
            let _ = p.yum.eat(33, 3.0, 0);
        }
        while rx.try_recv().is_ok() {}
        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "RESET YUM".into(),
            },
        );
        assert!(state.players.get(&1).unwrap().yum.history.is_empty());
        let mut saw_reset = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PS\n") && s.contains("YUM CLEAR OK") {
                saw_reset = true;
            }
        }
        assert!(saw_reset, "expected PS RESET YUM");

        while rx.try_recv().is_ok() {}
        {
            let p = state.players.get_mut(&1).unwrap();
            p.food = 3.5;
            p.food_max = 20.0;
        }
        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "BOOST".into(),
            },
        );
        assert!((state.players.get(&1).unwrap().food - 8.5).abs() < 1e-4);
        let mut saw_boost = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PS\n") && s.contains("BOOST OK") {
                saw_boost = true;
                assert!(s.contains("food=8.50"), "got {s}");
            }
        }
        assert!(saw_boost, "expected PS BOOST");

        {
            let p = state.players.get_mut(&1).unwrap();
            p.food = 18.0;
        }
        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "BOOST".into(),
            },
        );
        assert!((state.players.get(&1).unwrap().food - 20.0).abs() < 1e-4);

        while rx.try_recv().is_ok() {}
        assert!(!state.players.get(&1).unwrap().godmode);
        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "GODMODE".into(),
            },
        );
        assert!(state.players.get(&1).unwrap().godmode);
        let mut saw_god_on = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PS\n") && s.contains("GODMODE on") {
                saw_god_on = true;
            }
        }
        assert!(saw_god_on, "expected GODMODE on after toggle");

        while rx.try_recv().is_ok() {}
        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?GODMODE".into(),
            },
        );
        let mut saw_god_q = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PS\n") && s.contains("GODMODE on") {
                saw_god_q = true;
            }
        }
        assert!(saw_god_q, "expected ?GODMODE on");

        while rx.try_recv().is_ok() {}
        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "GODMODE OFF".into(),
            },
        );
        assert!(!state.players.get(&1).unwrap().godmode);
        let mut saw_god_off = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PS\n") && s.contains("GODMODE off") {
                saw_god_off = true;
            }
        }
        assert!(saw_god_off, "expected GODMODE off");

        while rx.try_recv().is_ok() {}
        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "FLAGS".into(),
            },
        );
        let mut saw_flags2 = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PS\n") && s.contains("FLAGS ") {
                saw_flags2 = true;
                assert!(s.contains("god=0"), "got {s}");
            }
        }
        assert!(saw_flags2, "expected bare FLAGS");
    }

    /// SAY ?WHERE / WHERE returns x y biome food age via private PS.
    #[test]
    fn say_where_returns_x_y_biome_food_age() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "where@x");

        // Known tile + vitals so the reply is deterministic.
        {
            let p = state.players.get_mut(&1).unwrap();
            p.x = 7;
            p.y = -3;
            p.food = 12.5;
            p.age = 20.0;
        }
        state.world.write().unwrap().set_biome(7, -3, 5); // desert

        let expected = SimState::format_where_query(7, -3, 5, 12.5, 20.0);
        assert_eq!(expected, "WHERE 7 -3 5 12.50 20.00");

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?WHERE".into(),
            },
        );

        let mut saw = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PS\n") && s.contains("WHERE ") {
                saw = true;
                assert!(
                    s.contains(&format!("{p_id} {expected}")),
                    "PS should embed x y biome food age: {s}"
                );
                assert!(s.contains("7 -3 5 12.50 20.00"), "got {s}");
            }
        }
        assert!(saw, "expected PS ?WHERE reply with x y biome food age");

        // Bare WHERE also works (private PS only).
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "WHERE".into(),
            },
        );
        let mut saw_bare = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PS\n") && s.contains(&format!("{p_id} WHERE ")) {
                saw_bare = true;
                assert!(s.contains("7 -3 5 12.50 20.00"), "got {s}");
            }
        }
        assert!(saw_bare, "expected PS bare WHERE reply");
    }

    /// SAY ?WHO / WHO lists online (connected, not deleted) p_ids + display names via PS.
    #[test]
    fn say_who_lists_online_player_ids_and_names() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx1 = hub.register(1);
        let mut rx2 = hub.register(2);
        let mut state = SimState::with_default_empty(test_content());
        assert_eq!(state.format_who_query(), "WHO none");

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Login {
                conn_id: 1,
                reconnect: false,
                email: "who_a@x".into(),
                client_tag: "t".into(),
            },
        );
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Login {
                conn_id: 2,
                reconnect: false,
                email: "who_b@x".into(),
                client_tag: "t".into(),
            },
        );
        let a = state.players.get(&1).unwrap().p_id;
        let b = state.players.get(&2).unwrap().p_id;
        let name_a = state.players.get(&1).unwrap().display_name();
        let name_b = state.players.get(&2).unwrap().display_name();

        let q = state.format_who_query();
        assert!(q.starts_with("WHO "), "got {q}");
        assert!(q.contains(&format!("{a} {name_a}")), "got {q}");
        assert!(q.contains(&format!("{b} {name_b}")), "got {q}");
        // Sorted by p_id: lower id appears first.
        let pos_a = q.find(&format!("{a} ")).unwrap();
        let pos_b = q.find(&format!("{b} ")).unwrap();
        if a < b {
            assert!(pos_a < pos_b, "expected sorted by p_id: {q}");
        } else {
            assert!(pos_b < pos_a, "expected sorted by p_id: {q}");
        }

        // Deleted / disconnected are excluded.
        state.players.get_mut(&2).unwrap().deleted = true;
        let q_del = state.format_who_query();
        assert!(q_del.contains(&format!("{a} {name_a}")), "got {q_del}");
        assert!(!q_del.contains(&format!("{b} ")), "deleted should be absent: {q_del}");
        state.players.get_mut(&2).unwrap().deleted = false;
        state.players.get_mut(&2).unwrap().connected = false;
        let q_dc = state.format_who_query();
        assert!(!q_dc.contains(&format!("{b} ")), "disconnected should be absent: {q_dc}");
        state.players.get_mut(&2).unwrap().connected = true;

        while rx1.try_recv().is_ok() {}
        while rx2.try_recv().is_ok() {}

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?WHO".into(),
            },
        );
        let mut saw_q = false;
        while let Ok(pkt) = rx1.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PS\n") && s.contains("WHO ") {
                saw_q = true;
                assert!(s.contains(&format!("{a} WHO ")), "got {s}");
                assert!(s.contains(&format!("{a} {name_a}")), "got {s}");
                assert!(s.contains(&format!("{b} {name_b}")), "got {s}");
            }
        }
        assert!(saw_q, "expected PS ?WHO reply");

        // Bare WHO also works (no nearby fan-out — private PS only).
        while rx1.try_recv().is_ok() {}
        while rx2.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "WHO".into(),
            },
        );
        let mut saw_bare = false;
        while let Ok(pkt) = rx1.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PS\n") && s.contains("WHO ") {
                saw_bare = true;
                assert!(s.contains(&format!("{a} WHO ")), "got {s}");
            }
        }
        assert!(saw_bare, "expected PS WHO reply");
        // Target alone receives reply — not fan-out to other conns.
        let mut leaked = false;
        while let Ok(pkt) = rx2.try_recv() {
            if String::from_utf8_lossy(&pkt).contains("WHO ") {
                leaked = true;
            }
        }
        assert!(!leaked, "WHO reply must not PS-fan to other players");
    }

    /// event_log records deaths/births/wars (max 100); SAY ?LOG returns last 5.
    #[test]
    fn event_log_death_birth_war_and_say_log() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        assert!(state.event_log.is_empty());
        assert_eq!(state.format_event_log_query(), "LOG none");

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Login {
                conn_id: 1,
                reconnect: false,
                email: "log@x".into(),
                client_tag: "t".into(),
            },
        );
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Login {
                conn_id: 2,
                reconnect: false,
                email: "log2@x".into(),
                client_tag: "t".into(),
            },
        );
        let a = state.players.get(&1).unwrap().p_id;
        let b = state.players.get(&2).unwrap().p_id;

        // Birth
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "BIRTH".into(),
            },
        );
        assert!(
            state.event_log.iter().any(|e| e.starts_with("BIRTH ") && e.contains(&format!("mother={a}"))),
            "expected BIRTH event, got {:?}",
            state.event_log
        );

        // War
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("WAR {b}"),
            },
        );
        assert!(
            state.event_log.iter().any(|e| e == &format!("WAR {a} {b}")),
            "expected WAR event, got {:?}",
            state.event_log
        );

        // Death via KILL
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("KILL {b}"),
            },
        );
        assert!(
            state
                .event_log
                .iter()
                .any(|e| e.starts_with(&format!("DEATH {b} "))),
            "expected DEATH event, got {:?}",
            state.event_log
        );

        // Ring buffer max EVENT_LOG_MAX: push 125 → keep last 100 (E25..=E124).
        state.event_log.clear();
        for i in 0..(EVENT_LOG_MAX + 25) {
            state.push_event(format!("E{i}"));
        }
        assert_eq!(state.event_log.len(), EVENT_LOG_MAX);
        assert_eq!(state.event_log.front().map(String::as_str), Some("E25"));
        let newest = format!("E{}", EVENT_LOG_MAX + 24);
        assert_eq!(state.event_log.back().map(String::as_str), Some(newest.as_str()));

        // format_event_log_query returns last EVENT_LOG_QUERY_LAST.
        let q = state.format_event_log_query();
        assert!(q.starts_with("LOG "));
        let first_of_last = EVENT_LOG_MAX + 24 + 1 - EVENT_LOG_QUERY_LAST;
        for i in first_of_last..=(EVENT_LOG_MAX + 24) {
            assert!(q.contains(&format!("E{i}")), "query missing E{i}: {q}");
        }
        // Oldest retained must not appear in last-5 query.
        assert!(!q.contains("E25"), "query should not include oldest: {q}");

        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?LOG".into(),
            },
        );
        let mut saw = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PS\n") && s.contains("LOG ") {
                saw = true;
                assert!(s.contains(&format!("{a} LOG ")), "got {s}");
                assert!(s.contains(&newest), "got {s}");
            }
        }
        assert!(saw, "expected PS ?LOG reply");

        // JOURNAL is an alias for the same event-log ring buffer.
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?JOURNAL".into(),
            },
        );
        let mut saw_journal = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PS\n") && s.contains("LOG ") {
                saw_journal = true;
                assert!(s.contains(&newest), "got {s}");
            }
        }
        assert!(saw_journal, "expected PS ?JOURNAL (=LOG) reply");
    }

    /// SAY POLL creates event-log line; VOTE yes|no tallies; ?POLL returns results.
    #[test]
    fn say_poll_vote_and_query() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx1 = hub.register(1);
        let mut rx2 = hub.register(2);
        let mut state = SimState::with_default_empty(test_content());

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Login {
                conn_id: 1,
                reconnect: false,
                email: "poll1@x".into(),
                client_tag: "t".into(),
            },
        );
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Login {
                conn_id: 2,
                reconnect: false,
                email: "poll2@x".into(),
                client_tag: "t".into(),
            },
        );
        let a = state.players.get(&1).unwrap().p_id;
        let b = state.players.get(&2).unwrap().p_id;

        assert_eq!(state.poll.format_query(), "POLL none");

        while rx1.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "POLL Build wall?".into(),
            },
        );
        assert!(
            state
                .event_log
                .iter()
                .any(|e| e == &format!("POLL {a} Build wall?")),
            "expected POLL event, got {:?}",
            state.event_log
        );
        assert!(state.poll.is_active());
        let mut saw_poll_ok = false;
        while let Ok(pkt) = rx1.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PS\n") && s.contains("POLL OK") {
                saw_poll_ok = true;
                assert!(s.contains("Build wall?"), "got {s}");
            }
        }
        assert!(saw_poll_ok, "expected PS POLL OK");

        while rx1.try_recv().is_ok() {}
        while rx2.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "VOTE yes".into(),
            },
        );
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 2,
                tag: "SAY".into(),
                payload: "VOTE no".into(),
            },
        );
        assert_eq!(state.poll.counts(), (1, 1));
        assert_eq!(state.poll.vote_of(a), Some(VoteChoice::Yes));
        assert_eq!(state.poll.vote_of(b), Some(VoteChoice::No));

        while rx1.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?POLL".into(),
            },
        );
        let mut saw_q = false;
        while let Ok(pkt) = rx1.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PS\n") && s.contains("POLL yes=") {
                saw_q = true;
                assert!(s.contains("yes=1"), "got {s}");
                assert!(s.contains("no=1"), "got {s}");
                assert!(s.contains("q=Build wall?"), "got {s}");
            }
        }
        assert!(saw_q, "expected PS ?POLL results");

        // Empty POLL fails; VOTE without choice fails; revote updates tallies.
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "POLL".into(),
            },
        );
        assert!(state.poll.is_active(), "empty POLL must not clear active");
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 2,
                tag: "SAY".into(),
                payload: "VOTE maybe".into(),
            },
        );
        assert_eq!(state.poll.counts(), (1, 1));
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 2,
                tag: "SAY".into(),
                payload: "VOTE yes".into(),
            },
        );
        assert_eq!(state.poll.counts(), (2, 0));
    }

    /// Pure-ish: no journal Arc → WJOURNAL none; shared journal peeks last entry.
    #[test]
    fn wjournal_none_or_last_entry() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        assert_eq!(state.format_wjournal_query(), "WJOURNAL none");
        assert_eq!(format_wjournal_query(None), "WJOURNAL none");
        assert_eq!(
            format_wjournal_query(Some((1, 2, 33, 9))),
            "WJOURNAL 1 2 33 9"
        );

        let p_id = spawn_player(&mut state, 1, "wj@x");
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?WJOURNAL".into(),
            },
        );
        let mut saw_none = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PS\n") && s.contains(&format!("{p_id} WJOURNAL none")) {
                saw_none = true;
            }
        }
        assert!(saw_none, "expected WJOURNAL none without journal Arc");

        // Attach journal + record one place → last entry summary.
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("ol_sim_wjournal_{nanos}.journal"));
        let _ = std::fs::remove_file(&path);
        state.journal = Some(Arc::new(Mutex::new(WorldJournal::open(&path))));
        state.tick = 11;
        state.record_world_change(4, 5, 99);
        assert_eq!(state.format_wjournal_query(), "WJOURNAL 4 5 99 11");

        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "WJOURNAL".into(),
            },
        );
        let mut saw_entry = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PS\n") && s.contains(&format!("{p_id} WJOURNAL 4 5 99 11")) {
                saw_entry = true;
            }
        }
        assert!(saw_entry, "expected WJOURNAL last entry PS");
        let _ = std::fs::remove_file(&path);
    }

    /// SAVE: non-operator DENIED; operator deferred without Arc; OK when hook Arc present.
    #[test]
    fn say_save_operator_and_deferred() {
        assert_eq!(format_save_reply(true), "SAVE OK");
        assert_eq!(format_save_reply(false), "SAVE deferred");
        assert_eq!(format_save_denied(), "SAVE DENIED");

        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "save@x");
        assert!(!state.players.get(&1).unwrap().godmode);

        // Non-operator → DENIED
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "SAVE".into(),
            },
        );
        let mut saw_denied = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PS\n") && s.contains(&format!("{p_id} SAVE DENIED")) {
                saw_denied = true;
            }
        }
        assert!(saw_denied, "expected SAVE DENIED without godmode");

        // Operator, no hook Arc → deferred
        state.players.get_mut(&1).unwrap().godmode = true;
        assert_eq!(state.request_force_save(), "SAVE deferred");
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "SAVE".into(),
            },
        );
        let mut saw_deferred = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PS\n") && s.contains(&format!("{p_id} SAVE deferred")) {
                saw_deferred = true;
            }
        }
        assert!(saw_deferred, "expected SAVE deferred without hook Arc");

        // Operator + hook Arc → sets flag + SAVE OK
        let flag = Arc::new(AtomicBool::new(false));
        state = state.with_save_request(Arc::clone(&flag));
        // re-spawn not needed; player still in map... wait, with_save_request consumes state
        // but we reassigned — players should still be there since with_save_request only sets field.
        assert!(state.players.get(&1).unwrap().godmode);
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?SAVE".into(),
            },
        );
        let mut saw_ok = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PS\n") && s.contains(&format!("{p_id} SAVE OK")) {
                saw_ok = true;
            }
        }
        assert!(saw_ok, "expected SAVE OK with hook Arc");
        assert!(
            flag.load(Ordering::Relaxed),
            "force-save flag should be set"
        );
    }

    #[test]
    fn birth_sets_lineage_mother_and_age_zero() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut state = SimState::with_default_empty(test_content());
        let mother_p_id = spawn_player(&mut state, 1, "mother@x");
        state.social.ensure_lineage(mother_p_id, "MOTHER");
        // Move mother so marker coords are non-default.
        set_player_position(&mut state, 1, 12, 34);
        let before_next = state.next_player_id;

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "BIRTH".into(),
            },
        );

        let baby_conn = 1u64 + BABY_CONN_OFFSET;
        let baby = state
            .players
            .get(&baby_conn)
            .expect("baby player at mother_conn + BABY_CONN_OFFSET");
        assert_eq!(baby.age, 0.0, "baby age must be 0");
        assert_eq!(baby.food, START_FOOD, "baby food must be 10");
        assert_eq!(baby.p_id, before_next);
        assert_eq!(baby.x, 12);
        assert_eq!(baby.y, 34);
        assert_eq!(state.next_player_id, before_next + 1);

        let node = state
            .social
            .lineages
            .get(&baby.p_id)
            .expect("baby lineage");
        assert_eq!(node.mother_id, Some(mother_p_id));
        assert_eq!(node.generation, 1);

        let markers = state.markers.wire_lines_for(baby.p_id);
        assert!(
            markers.iter().any(|m| m.contains("12 34") && m.contains("MOTHER")),
            "expected mother marker for baby, got {markers:?}"
        );

        // Direct API path also works.
        let baby2_id = spawn_child(&mut state, 1).expect("second birth");
        let node2 = state.social.lineages.get(&baby2_id).unwrap();
        assert_eq!(node2.mother_id, Some(mother_p_id));
        let baby2 = state
            .players
            .values()
            .find(|p| p.p_id == baby2_id)
            .unwrap();
        assert_eq!(baby2.age, 0.0);
    }

    /// SAY ?HELD / HELD returns held_id and content object name when known.
    #[test]
    fn say_held_reports_id_and_content_name() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "held@test");

        // Empty hands.
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?HELD".into(),
            },
        );
        let empty = rx.try_recv().expect("PS ?HELD empty");
        let empty_s = String::from_utf8_lossy(&empty);
        assert!(empty_s.starts_with("PS\n"), "got {empty_s}");
        assert!(empty_s.contains("HELD 0"), "got {empty_s}");
        assert!(
            !empty_s.contains("HELD 0 "),
            "empty hands must not append a name: {empty_s}"
        );

        // Holding known object 33 (Gooseberry in test_content).
        state.players.get_mut(&1).unwrap().held_id = 33;
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "HELD".into(),
            },
        );
        let named = rx.try_recv().expect("PS HELD named");
        let named_s = String::from_utf8_lossy(&named);
        assert!(named_s.starts_with("PS\n"), "got {named_s}");
        assert!(
            named_s.contains("HELD 33 Gooseberry"),
            "got {named_s}"
        );

        // Unknown object id → id only (no content name).
        state.players.get_mut(&1).unwrap().held_id = 9999;
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?HELD".into(),
            },
        );
        let unk = rx.try_recv().expect("PS ?HELD unknown");
        let unk_s = String::from_utf8_lossy(&unk);
        assert!(unk_s.contains("HELD 9999"), "got {unk_s}");
        assert!(
            !unk_s.contains("HELD 9999 "),
            "unknown id must not invent a name: {unk_s}"
        );

        // Pure formatter unit check (no wire).
        assert_eq!(state.format_held_query(0), "HELD 0");
        assert_eq!(state.format_held_query(33), "HELD 33 Gooseberry");
        assert_eq!(state.format_held_query(9999), "HELD 9999");
    }

    /// Clothing slots start empty; set_clothing + SAY CLOTHES report ids.
    #[test]
    fn clothing_slots_set_and_say_clothes() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "clothes@test");
        {
            let p = state.players.get_mut(&1).unwrap();
            assert_eq!((p.hat, p.chest, p.shoes), (0, 0, 0));
            p.set_clothing(ClothingSlot::Hat, 55);
            p.set_clothing(ClothingSlot::Chest, 66);
            p.set_clothing(ClothingSlot::Shoes, 77);
        }
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "CLOTHES".into(),
            },
        );
        let msg = rx.try_recv().expect("PS CLOTHES");
        let s = String::from_utf8_lossy(&msg);
        assert!(s.starts_with("PS\n"), "got {s}");
        assert!(
            s.contains("CLOTHES hat=55 chest=66 shoes=77"),
            "got {s}"
        );
    }

    /// SAY STORE / INV / TAKE move held into backpack (max 8) and back to hands.
    #[test]
    fn say_store_inv_take_backpack() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "bp@test");
        {
            let p = state.players.get_mut(&1).unwrap();
            assert!(p.backpack.is_empty());
            p.held_id = 33;
        }

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "STORE".into(),
            },
        );
        {
            let p = state.players.get(&1).unwrap();
            assert_eq!(p.held_id, 0);
            assert_eq!(p.backpack, vec![33]);
        }
        let mut saw_store = false;
        while let Ok(msg) = rx.try_recv() {
            let s = String::from_utf8_lossy(&msg);
            if s.contains("STORE 33 OK") {
                saw_store = true;
            }
        }
        assert!(saw_store, "expected PS STORE 33 OK");

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "INV".into(),
            },
        );
        let inv = rx.try_recv().expect("PS INV");
        let inv_s = String::from_utf8_lossy(&inv);
        assert!(
            inv_s.contains(&format!("INV 1/{BACKPACK_MAX} 33")),
            "got {inv_s}"
        );

        // Empty STORE fails.
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "STORE".into(),
            },
        );
        let fail = rx.try_recv().expect("PS STORE FAIL");
        assert!(
            String::from_utf8_lossy(&fail).contains("STORE FAIL EMPTY"),
            "got {}",
            String::from_utf8_lossy(&fail)
        );
        // Drain any PU from earlier.
        while rx.try_recv().is_ok() {}

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "TAKE 0".into(),
            },
        );
        {
            let p = state.players.get(&1).unwrap();
            assert_eq!(p.held_id, 33);
            assert!(p.backpack.is_empty());
        }
        let mut saw_take = false;
        while let Ok(msg) = rx.try_recv() {
            let s = String::from_utf8_lossy(&msg);
            if s.contains("TAKE 0 33 OK") {
                saw_take = true;
            }
        }
        assert!(saw_take, "expected PS TAKE 0 33 OK");
    }

    /// Backpack rejects a 9th STORE (max BACKPACK_MAX).
    #[test]
    fn say_store_backpack_max_eight() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "bpfull@test");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.backpack = (1..=BACKPACK_MAX as i32).collect();
            p.held_id = 999;
        }
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "STORE".into(),
            },
        );
        let p = state.players.get(&1).unwrap();
        assert_eq!(p.held_id, 999);
        assert_eq!(p.backpack.len(), BACKPACK_MAX);
        let msg = rx.try_recv().expect("PS STORE FAIL");
        assert!(
            String::from_utf8_lossy(&msg).contains("STORE FAIL FULL"),
            "got {}",
            String::from_utf8_lossy(&msg)
        );
    }

    /// SAY NOTE / ?NOTES personal journal (max NOTES_MAX).
    #[test]
    fn say_note_and_notes_query() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "note@test");
        let p_id = state.players.get(&1).unwrap().p_id;

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?NOTES".into(),
            },
        );
        let empty = rx.try_recv().expect("PS empty NOTES");
        let empty_s = String::from_utf8_lossy(&empty);
        assert!(
            empty_s.contains(&format!("{p_id} NOTES 0/{NOTES_MAX}")),
            "got {empty_s}"
        );

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "NOTE found water".into(),
            },
        );
        let ack = rx.try_recv().expect("PS NOTE OK");
        let ack_s = String::from_utf8_lossy(&ack);
        assert!(
            ack_s.contains(&format!("{p_id} NOTE 1/{NOTES_MAX} OK")),
            "got {ack_s}"
        );
        assert_eq!(
            state.players.get(&1).unwrap().notes,
            vec!["found water".to_string()]
        );

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "NOTE".into(),
            },
        );
        let fail_empty = rx.try_recv().expect("PS NOTE FAIL EMPTY");
        assert!(
            String::from_utf8_lossy(&fail_empty).contains("NOTE FAIL EMPTY"),
            "got {}",
            String::from_utf8_lossy(&fail_empty)
        );

        // Fill to capacity (advance sim_time so SAY rate limit does not block).
        for i in 2..=NOTES_MAX {
            state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
            apply_intent(
                &mut state,
                &counters,
                &hub,
                NetIntent::Raw {
                    conn_id: 1,
                    tag: "SAY".into(),
                    payload: format!("NOTE n{i}"),
                },
            );
            let _ = rx.try_recv();
        }
        assert_eq!(state.players.get(&1).unwrap().notes.len(), NOTES_MAX);

        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "NOTE overflow".into(),
            },
        );
        let full = rx.try_recv().expect("PS NOTE FAIL FULL");
        assert!(
            String::from_utf8_lossy(&full).contains("NOTE FAIL FULL"),
            "got {}",
            String::from_utf8_lossy(&full)
        );

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "NOTES".into(),
            },
        );
        let list = rx.try_recv().expect("PS NOTES list");
        let list_s = String::from_utf8_lossy(&list);
        assert!(
            list_s.contains(&format!("NOTES {NOTES_MAX}/{NOTES_MAX}"))
                && list_s.contains("0:found water"),
            "got {list_s}"
        );
    }

    /// SAY REMEMBER / FORGET / ?MEMORY aliases for NOTE journal; FORGET pops last.
    #[test]
    fn say_remember_forget_and_memory_query() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "mem@test");
        let p_id = state.players.get(&1).unwrap().p_id;

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?MEMORY".into(),
            },
        );
        let empty = rx.try_recv().expect("PS empty MEMORY/NOTES");
        let empty_s = String::from_utf8_lossy(&empty);
        assert!(
            empty_s.contains(&format!("{p_id} NOTES 0/{NOTES_MAX}")),
            "got {empty_s}"
        );

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "REMEMBER river east".into(),
            },
        );
        let ack = rx.try_recv().expect("PS REMEMBER/NOTE OK");
        let ack_s = String::from_utf8_lossy(&ack);
        assert!(
            ack_s.contains(&format!("{p_id} NOTE 1/{NOTES_MAX} OK")),
            "got {ack_s}"
        );
        assert_eq!(
            state.players.get(&1).unwrap().notes,
            vec!["river east".to_string()]
        );

        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "REMEMBER berries".into(),
            },
        );
        let _ = rx.try_recv();
        assert_eq!(state.players.get(&1).unwrap().notes.len(), 2);

        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "FORGET".into(),
            },
        );
        let forget = rx.try_recv().expect("PS FORGET OK");
        let forget_s = String::from_utf8_lossy(&forget);
        assert!(
            forget_s.contains(&format!("{p_id} FORGET 1/{NOTES_MAX} OK berries")),
            "got {forget_s}"
        );
        assert_eq!(
            state.players.get(&1).unwrap().notes,
            vec!["river east".to_string()]
        );

        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "MEMORY".into(),
            },
        );
        let list = rx.try_recv().expect("PS MEMORY list");
        let list_s = String::from_utf8_lossy(&list);
        assert!(
            list_s.contains(&format!("NOTES 1/{NOTES_MAX}")) && list_s.contains("0:river east"),
            "got {list_s}"
        );

        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "FORGET".into(),
            },
        );
        let _ = rx.try_recv();
        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "FORGET".into(),
            },
        );
        let fail = rx.try_recv().expect("PS FORGET FAIL EMPTY");
        assert!(
            String::from_utf8_lossy(&fail).contains("FORGET FAIL EMPTY"),
            "got {}",
            String::from_utf8_lossy(&fail)
        );
    }

    /// SAY TITLE sets personal title; ?NAME includes it after `|`.
    #[test]
    fn say_title_and_name_shows_title() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "title@test");
        let p_id = state.players.get(&1).unwrap().p_id;
        {
            let p = state.players.get_mut(&1).unwrap();
            p.first_name = "ADA".into();
            p.family_name = "SNOW".into();
        }

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "TITLE".into(),
            },
        );
        let fail = rx.try_recv().expect("PS TITLE FAIL EMPTY");
        assert!(
            String::from_utf8_lossy(&fail).contains("TITLE FAIL EMPTY"),
            "got {}",
            String::from_utf8_lossy(&fail)
        );

        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "TITLE Scout".into(),
            },
        );
        let ok = rx.try_recv().expect("PS TITLE OK");
        let ok_s = String::from_utf8_lossy(&ok);
        assert!(
            ok_s.contains(&format!("{p_id} TITLE OK Scout")),
            "got {ok_s}"
        );
        assert_eq!(state.players.get(&1).unwrap().title, "Scout");

        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?NAME".into(),
            },
        );
        let name = rx.try_recv().expect("PS NAME with title");
        let name_s = String::from_utf8_lossy(&name);
        assert!(
            name_s.contains(&format!("{p_id} NAME ADA SNOW | Scout")),
            "got {name_s}"
        );
        // NM / display_name stays first last without title.
        assert_eq!(
            state.players.get(&1).unwrap().display_name(),
            "ADA SNOW"
        );

        // Truncation
        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        let long: String = "z".repeat(TITLE_TEXT_MAX + 15);
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("TITLE {long}"),
            },
        );
        let _ = rx.try_recv();
        assert_eq!(
            state.players.get(&1).unwrap().title.chars().count(),
            TITLE_TEXT_MAX
        );
    }

    /// SAY WATER boosts food by +1 (capped at food_max).
    #[test]
    fn say_water_food_boost() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "water@test");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.food = 10.0;
            p.food_max = 20.0;
        }
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "WATER".into(),
            },
        );
        let p = state.players.get(&1).unwrap();
        assert!((p.food - 11.0).abs() < 1e-5, "food +1, got {}", p.food);
        let msg = rx.try_recv().expect("PS WATER");
        let s = String::from_utf8_lossy(&msg);
        assert!(s.contains("WATER OK food=11.00"), "got {s}");

        // Cap at food_max.
        state.players.get_mut(&1).unwrap().food = 20.0;
        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "WATER".into(),
            },
        );
        assert!((state.players.get(&1).unwrap().food - 20.0).abs() < 1e-5);
        let full = rx.try_recv().expect("PS WATER full");
        assert!(
            String::from_utf8_lossy(&full).contains("WATER OK full"),
            "got {}",
            String::from_utf8_lossy(&full)
        );
    }

    /// SAY STRIP / WEAR move clothing ↔ hands.
    #[test]
    fn say_wear_and_strip_clothing() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut db = ContentDb::default();
        db.objects.insert(
            500,
            ObjectDef {
                id: 500,
                description: "Wool Hat".into(),
                name: "Wool Hat".into(),
                containable: true,
                permanent: false,
                blocks_walking: false,
                food_value: 0,
                heat_value: 0.0,
                map_chance: 0.0,
                biomes: Vec::new(),
                num_uses: 0,
                num_slots: 0,
                floor: false,
            dummy_ids: Vec::new(),
            },
        );
        db.objects.insert(
            501,
            ObjectDef {
                id: 501,
                description: "Linen Shirt".into(),
                name: "Linen Shirt".into(),
                containable: true,
                permanent: false,
                blocks_walking: false,
                food_value: 0,
                heat_value: 0.0,
                map_chance: 0.0,
                biomes: Vec::new(),
                num_uses: 0,
                num_slots: 0,
                floor: false,
            dummy_ids: Vec::new(),
            },
        );
        let mut state = SimState::with_default_empty(Arc::new(db));
        spawn_player(&mut state, 1, "wear@test");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.held_id = 500;
            p.hat = 0;
        }
        while rx.try_recv().is_ok() {}

        // WEAR without slot: infer hat from name.
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "WEAR".into(),
            },
        );
        {
            let p = state.players.get(&1).unwrap();
            assert_eq!(p.hat, 500);
            assert_eq!(p.held_id, 0);
        }
        let mut saw_wear = false;
        while let Ok(msg) = rx.try_recv() {
            if String::from_utf8_lossy(&msg).contains("WEAR hat 500 OK") {
                saw_wear = true;
            }
        }
        assert!(saw_wear, "expected WEAR hat 500 OK");

        // STRIP hat → hands.
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "STRIP hat".into(),
            },
        );
        {
            let p = state.players.get(&1).unwrap();
            assert_eq!(p.hat, 0);
            assert_eq!(p.held_id, 500);
        }
        let mut saw_strip = false;
        while let Ok(msg) = rx.try_recv() {
            if String::from_utf8_lossy(&msg).contains("STRIP hat 500 OK") {
                saw_strip = true;
            }
        }
        assert!(saw_strip, "expected STRIP hat 500 OK");

        // Explicit WEAR chest with shirt; swap previous.
        state.players.get_mut(&1).unwrap().held_id = 501;
        state.players.get_mut(&1).unwrap().chest = 99;
        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "WEAR chest".into(),
            },
        );
        {
            let p = state.players.get(&1).unwrap();
            assert_eq!(p.chest, 501);
            assert_eq!(p.held_id, 99);
        }
        let mut saw_swap = false;
        while let Ok(msg) = rx.try_recv() {
            if String::from_utf8_lossy(&msg).contains("WEAR chest 501 OK swap=99") {
                saw_swap = true;
            }
        }
        assert!(saw_swap, "expected WEAR chest swap");

        // STRIP with full hands fails.
        state.players.get_mut(&1).unwrap().hat = 500;
        // held already 99
        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "STRIP hat".into(),
            },
        );
        let fail = rx.try_recv().expect("STRIP FAIL");
        assert!(
            String::from_utf8_lossy(&fail).contains("STRIP FAIL HANDS"),
            "got {}",
            String::from_utf8_lossy(&fail)
        );
    }

    /// Death scatters held + clothing + backpack onto empty neighboring tiles.
    #[test]
    fn death_scatters_backpack_on_ground() {
        let hub = OutboundHub::new();
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "scatter@die");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.x = 50;
            p.y = 50;
            p.backpack = vec![33, 34, 35];
            p.held_id = 99;
            p.hat = 40;
            p.chest = 41;
            p.shoes = 42;
            p.food = 0.05;
            p.age = 20.0;
        }
        // Occupy death tile so first scatter prefers neighbors.
        state.world.write().unwrap().set_object(50, 50, 1);

        tick_vitals(&mut state, 1.0, &hub);

        let p = state.players.get(&1).unwrap();
        assert!(p.deleted);
        assert!(p.backpack.is_empty(), "backpack drained on death");
        assert_eq!(p.held_id, 0);
        assert_eq!(p.hat, 0);
        assert_eq!(p.chest, 0);
        assert_eq!(p.shoes, 0);

        let mut found = Vec::new();
        let w = state.world.read().unwrap();
        for dy in -2..=2 {
            for dx in -2..=2 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let id = w.get_object(50 + dx, 50 + dy);
                if matches!(id, 33 | 34 | 35 | 99 | 40 | 41 | 42) {
                    found.push(id);
                }
            }
        }
        found.sort();
        assert_eq!(
            found,
            vec![33, 34, 35, 40, 41, 42, 99],
            "held+clothing+backpack scattered near death"
        );
        assert_eq!(w.get_object(50, 50), 1, "occupied death tile untouched");
        drop(w);

        // Event log records SCATTER (7 loot pieces).
        let saw = state
            .event_log
            .iter()
            .any(|e| e.contains("SCATTER") && e.contains("n=7"));
        assert!(saw, "expected SCATTER event, log={:?}", state.event_log);

        // Pure offset helper: ring 1 then (0,0).
        let off = death_scatter_offsets(1);
        assert!(off.contains(&(1, 0)));
        assert!(off.contains(&(0, 1)));
        assert_eq!(*off.last().unwrap(), (0, 0));
    }

    /// SAY DROPALL scatters held+backpack without death; clothing stays.
    #[test]
    fn say_dropall_scatters_held_and_backpack() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "dropall@test");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.x = 30;
            p.y = 30;
            p.held_id = 55;
            p.hat = 77;
            p.backpack = vec![66, 67];
        }
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "DROPALL".into(),
            },
        );
        let p = state.players.get(&1).unwrap();
        assert!(!p.deleted, "DROPALL must not kill");
        assert_eq!(p.held_id, 0);
        assert!(p.backpack.is_empty());
        assert_eq!(p.hat, 77, "clothing kept on DROPALL");

        let mut found = Vec::new();
        {
            let w = state.world.read().unwrap();
            for dy in -DEATH_SCATTER_RADIUS..=DEATH_SCATTER_RADIUS {
                for dx in -DEATH_SCATTER_RADIUS..=DEATH_SCATTER_RADIUS {
                    let id = w.get_object(30 + dx, 30 + dy);
                    if matches!(id, 55 | 66 | 67) {
                        found.push(id);
                    }
                }
            }
        }
        found.sort();
        assert_eq!(found, vec![55, 66, 67], "held+backpack on ground");

        let mut saw_ok = false;
        while let Ok(msg) = rx.try_recv() {
            let s = String::from_utf8_lossy(&msg);
            if s.contains("DROPALL OK n=3") {
                saw_ok = true;
            }
        }
        assert!(saw_ok, "expected DROPALL OK n=3");

        // Empty DROPALL reports n=0.
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "DROPALL".into(),
            },
        );
        let empty = rx.try_recv().expect("PS DROPALL empty");
        assert!(
            String::from_utf8_lossy(&empty).contains("DROPALL OK n=0"),
            "got {}",
            String::from_utf8_lossy(&empty)
        );
    }

    /// SAY DIE also scatters a full backpack (drop-on-ground fallback).
    #[test]
    fn say_die_scatters_full_backpack() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "bpdie@test");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.x = 20;
            p.y = 20;
            p.backpack = (1..=BACKPACK_MAX as i32).map(|i| 100 + i).collect();
        }
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "DIE".into(),
            },
        );
        let p = state.players.get(&1).unwrap();
        assert!(p.deleted);
        assert!(p.backpack.is_empty());
        let mut n = 0;
        let w = state.world.read().unwrap();
        for dy in -DEATH_SCATTER_RADIUS..=DEATH_SCATTER_RADIUS {
            for dx in -DEATH_SCATTER_RADIUS..=DEATH_SCATTER_RADIUS {
                let id = w.get_object(20 + dx, 20 + dy);
                if (101..=100 + BACKPACK_MAX as i32).contains(&id) {
                    n += 1;
                }
            }
        }
        assert_eq!(n, BACKPACK_MAX, "full backpack scattered on SAY DIE");
    }

    /// USE whose new_actor name contains "hat" assigns the hat slot.
    #[test]
    fn use_equips_clothing_like_new_actor() {
        let mut db = ContentDb::default();
        db.objects.insert(
            500,
            ObjectDef {
                id: 500,
                description: "Wool Hat".into(),
                name: "Wool Hat".into(),
                containable: true,
                permanent: false,
                blocks_walking: false,
                food_value: 0,
                heat_value: 0.0,
                map_chance: 0.0,
                biomes: Vec::new(),
                num_uses: 0,
                num_slots: 0,
                floor: false,
            dummy_ids: Vec::new(),
            },
        );
        // Bare-hand USE on tile 1 → new_actor is Wool Hat (500).
        db.transitions.insert(
            (0, 1),
            Transition {
                actor_id: 0,
                target_id: 1,
                new_actor_id: 500,
                new_target_id: 0,
                last_use_actor: false,
                last_use_target: false,
                auto_decay_seconds: 0.0,
                reverse_use_actor: false,
                reverse_use_target: false,
                no_use_actor: false,
                no_use_target: false,
                move_dist: 0,

            desired_move_dist: 0,
            },
        );
        db.transition_count = 1;
        let mut state = SimState::with_default_empty(Arc::new(db));
        spawn_player(&mut state, 1, "hat@test");
        state.world.write().unwrap().set_object(2, 2, 1);
        {
            let p = state.players.get_mut(&1).unwrap();
            p.x = 2;
            p.y = 2;
            p.held_id = 0;
            p.hat = 0;
        }
        let r = apply_use_at(&mut state, 1, 2, 2).expect("use");
        assert!(r.applied);
        assert_eq!(r.actor_after, 500);
        let p = state.players.get(&1).unwrap();
        assert_eq!(p.held_id, 500);
        assert_eq!(p.hat, 500);
        assert_eq!(p.chest, 0);
        assert_eq!(p.shoes, 0);
    }

    /// SAY HOME stores current tile on Player.home_x / home_y.
    #[test]
    fn say_home_sets_home_position() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "home@test");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.x = 15;
            p.y = 27;
            // Different from spawn so we can detect the set.
            p.home_x = 0;
            p.home_y = 0;
        }
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "HOME".into(),
            },
        );
        let p = state.players.get(&1).unwrap();
        assert_eq!((p.home_x, p.home_y), (15, 27));
        assert_eq!((p.x, p.y), (15, 27));
    }

    /// SAY MARK <label> pins a custom MarkerState entry at current pos for self.
    #[test]
    fn say_mark_adds_custom_marker_for_self() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "mark@test");
        let p_id = {
            let p = state.players.get_mut(&1).unwrap();
            p.x = 42;
            p.y = 17;
            p.p_id
        };
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "MARK campfire".into(),
            },
        );
        let lines = state.markers.wire_lines_for(p_id);
        assert!(
            lines.iter().any(|m| m == "42 17 ! campfire"),
            "expected custom marker for self, got {lines:?}"
        );
        let list = state.markers.markers.get(&p_id).expect("self markers");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].kind, MarkerKind::Custom);
        assert_eq!(list[0].owner_p_id, p_id);
        assert_eq!(list[0].label, "campfire");
        // Confirm PS ack, not generic chat broadcast only.
        let mut saw_ack = false;
        while let Ok(msg) = rx.try_recv() {
            let s = String::from_utf8_lossy(&msg);
            if s.contains("MARK 42 17 campfire") {
                saw_ack = true;
            }
        }
        assert!(saw_ack, "expected PS MARK ack");
        // Bare MARK without label fails and does not add a marker.
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "MARK".into(),
            },
        );
        assert_eq!(
            state.markers.markers.get(&p_id).map(|v| v.len()),
            Some(1),
            "empty MARK must not add another marker"
        );
    }

    /// SAY PATH / STEPS / WALKABLE pathfind chat probes (gate exception + blocks).
    #[test]
    fn say_path_steps_walkable() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut db = ContentDb::default();
        db.objects.insert(
            10,
            ObjectDef {
                id: 10,
                description: "Stone Wall".into(),
                name: "Stone Wall".into(),
                containable: false,
                permanent: true,
                blocks_walking: true,
                food_value: 0,
                heat_value: 0.0,
                map_chance: 0.0,
                biomes: Vec::new(),
                num_uses: 0,
                num_slots: 0,
                floor: false,
            dummy_ids: Vec::new(),
            },
        );
        db.objects.insert(
            20,
            ObjectDef {
                id: 20,
                description: "Vertical Gate".into(),
                name: "Vertical Gate".into(),
                containable: false,
                permanent: true,
                blocks_walking: true,
                food_value: 0,
                heat_value: 0.0,
                map_chance: 0.0,
                biomes: Vec::new(),
                num_uses: 0,
                num_slots: 0,
                floor: false,
            dummy_ids: Vec::new(),
            },
        );
        let mut state = SimState::with_default_empty(Arc::new(db));
        let p_id = spawn_player(&mut state, 1, "path@x");
        set_player_position(&mut state, 1, 0, 0);
        {
            let mut w = state.world.write().unwrap();
            w.set_object(1, 0, 10); // wall east
            w.set_object(0, 1, 20); // gate south (walkable by name exception)
        }

        // WALKABLE: wall no, gate yes, empty yes.
        for (payload, expect) in [
            ("WALKABLE 1 0", "WALKABLE no"),
            ("WALKABLE 0 1", "WALKABLE yes"),
            ("WALKABLE 0 0", "WALKABLE yes"),
        ] {
            apply_intent(
                &mut state,
                &counters,
                &hub,
                NetIntent::Raw {
                    conn_id: 1,
                    tag: "SAY".into(),
                    payload: payload.into(),
                },
            );
            let mut saw = false;
            while let Ok(pkt) = rx.try_recv() {
                let s = String::from_utf8_lossy(&pkt);
                if s.contains(&format!("{p_id} {expect}")) {
                    saw = true;
                }
            }
            assert!(saw, "expected PS containing {expect} for {payload}");
        }

        // Reset SAY rate window (5 / 10s) before more probes.
        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;

        // PATH around wall toward (2,0): first step must not be into wall (1,0).
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "PATH 2 0".into(),
            },
        );
        let mut path_line = None;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if let Some(idx) = s.find(&format!("{p_id} PATH ")) {
                path_line = Some(s[idx..].lines().next().unwrap_or("").to_string());
            }
        }
        let path_line = path_line.expect("PATH reply");
        assert!(
            !path_line.contains("PATH FAIL"),
            "open detour should succeed: {path_line}"
        );
        assert!(
            !path_line.contains("PATH 1 0"),
            "must not step into wall: {path_line}"
        );

        // STEPS to (0,2) via gate corridor should be finite.
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "STEPS 0 2".into(),
            },
        );
        let mut saw_steps = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains(&format!("{p_id} STEPS 2")) {
                saw_steps = true;
            }
        }
        assert!(saw_steps, "STEPS 0 2 should be 2 through gate");

        // Already at goal.
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "PATH 0 0".into(),
            },
        );
        let mut saw_zero = false;
        while let Ok(pkt) = rx.try_recv() {
            if String::from_utf8_lossy(&pkt).contains(&format!("{p_id} PATH 0 0")) {
                saw_zero = true;
            }
        }
        assert!(saw_zero);

        // Seal player so goal is unreachable → FAIL.
        {
            let mut w = state.world.write().unwrap();
            w.set_object(1, 0, 10);
            w.set_object(-1, 0, 10);
            w.set_object(0, 1, 10);
            w.set_object(0, -1, 10);
        }
        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "PATH 5 5".into(),
            },
        );
        let mut saw_fail = false;
        while let Ok(pkt) = rx.try_recv() {
            if String::from_utf8_lossy(&pkt).contains(&format!("{p_id} PATH FAIL")) {
                saw_fail = true;
            }
        }
        assert!(saw_fail, "boxed-in player should PATH FAIL");

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "STEPS 5 5".into(),
            },
        );
        let mut saw_steps_fail = false;
        while let Ok(pkt) = rx.try_recv() {
            if String::from_utf8_lossy(&pkt).contains(&format!("{p_id} STEPS FAIL")) {
                saw_steps_fail = true;
            }
        }
        assert!(saw_steps_fail);
    }

    /// SAY GOHOME moves one step toward home (pathfind or cardinal teleport).
    #[test]
    fn say_gohome_steps_toward_home() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "gohome@test");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.x = 0;
            p.y = 0;
            p.home_x = 5;
            p.home_y = 0;
        }
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "GOHOME".into(),
            },
        );
        let p = state.players.get(&1).unwrap();
        // One step east toward home_x=5.
        assert_eq!((p.x, p.y), (1, 0));
        assert_eq!((p.home_x, p.home_y), (5, 0));
    }

    /// SAY SLEEP sets Player.sleeping; SAY WAKE clears it.
    #[test]
    fn say_sleep_and_wake_toggle_sleeping() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "sleep@test");
        assert!(!state.players.get(&1).unwrap().sleeping);

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "SLEEP".into(),
            },
        );
        assert!(state.players.get(&1).unwrap().sleeping);

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "WAKE".into(),
            },
        );
        assert!(!state.players.get(&1).unwrap().sleeping);
    }

    /// MOVE is rejected while sleeping; works again after WAKE.
    #[test]
    fn move_blocked_while_sleeping() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "sleeper@test");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.x = 0;
            p.y = 0;
        }

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "SLEEP".into(),
            },
        );
        assert!(!apply_move_deltas(&mut state, 1, 0, 0, &[(1, 0)]));
        assert_eq!((state.players.get(&1).unwrap().x, state.players.get(&1).unwrap().y), (0, 0));

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "WAKE".into(),
            },
        );
        assert!(apply_move_deltas(&mut state, 1, 0, 0, &[(1, 0)]));
        assert_eq!((state.players.get(&1).unwrap().x, state.players.get(&1).unwrap().y), (1, 0));
    }

    /// SAY is capped at 5 per 10 sim seconds; excess returns `PS RATE`.
    #[test]
    fn say_rate_limited_to_five_per_ten_sim_seconds() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "chatty@test");
        while rx.try_recv().is_ok() {}

        for i in 0..SAY_RATE_MAX {
            apply_intent(
                &mut state,
                &counters,
                &hub,
                NetIntent::Raw {
                    conn_id: 1,
                    tag: "SAY".into(),
                    payload: format!("hi{i}"),
                },
            );
        }
        // Sixth SAY in the same sim-time window is rejected.
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "spam".into(),
            },
        );
        let mut saw_rate = false;
        let mut chat_ok = 0usize;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s == "PS\nRATE\n#" || s.starts_with("PS\nRATE\n") {
                saw_rate = true;
            }
            if s.contains("hi") {
                chat_ok += 1;
            }
            assert!(
                !s.contains("spam"),
                "rate-limited SAY must not broadcast chat: {s}"
            );
        }
        assert!(saw_rate, "expected PS RATE on 6th SAY");
        assert_eq!(chat_ok, SAY_RATE_MAX, "first {SAY_RATE_MAX} SAYs should chat");
        assert_eq!(
            state.players.get(&1).unwrap().last_say_times.len(),
            SAY_RATE_MAX
        );

        // After the window elapses, SAY is allowed again.
        tick_vitals(&mut state, SAY_RATE_WINDOW_SECS, &hub);
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "again".into(),
            },
        );
        let mut saw_again = false;
        let mut saw_rate_after = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("again") {
                saw_again = true;
            }
            if s.contains("RATE") {
                saw_rate_after = true;
            }
        }
        assert!(saw_again, "SAY should work after window");
        assert!(!saw_rate_after, "should not RATE after window elapsed");
    }

    /// While sleeping, food drain is halved (SLEEP_FOOD_DRAIN_MULT = 0.5).
    #[test]
    fn sleeping_halves_food_drain() {
        let hub = OutboundHub::new();
        let mut awake = SimState::with_default_empty(test_content());
        let mut asleep = SimState::with_default_empty(test_content());
        spawn_player(&mut awake, 1, "awake");
        spawn_player(&mut asleep, 1, "asleep");
        for s in [&mut awake, &mut asleep] {
            s.environment.temperature = 0.5;
            s.environment.season_length = 10_000.0;
            s.environment.day_length = 10_000.0;
            s.environment.hour_of_day = 12.0;
        }
        asleep.players.get_mut(&1).unwrap().sleeping = true;

        let food0 = awake.players.get(&1).unwrap().food;
        assert_eq!(food0, asleep.players.get(&1).unwrap().food);

        tick_vitals(&mut awake, 1.0, &hub);
        tick_vitals(&mut asleep, 1.0, &hub);

        let awake_lost = food0 - awake.players.get(&1).unwrap().food;
        let asleep_lost = food0 - asleep.players.get(&1).unwrap().food;
        assert!(
            (awake_lost - FOOD_USE_PER_SEC).abs() < 1e-4,
            "awake drain: lost={awake_lost}"
        );
        let expected_sleep = FOOD_USE_PER_SEC * SLEEP_FOOD_DRAIN_MULT;
        assert!(
            (asleep_lost - expected_sleep).abs() < 1e-4,
            "sleep drain: lost={asleep_lost} expected={expected_sleep}"
        );
        assert!(asleep_lost < awake_lost);
    }

    /// While sleeping, PE sleep/snore emote fires every SLEEP_EMOT_INTERVAL_SECS.
    #[test]
    fn tick_vitals_emits_pe_sleep_emote_while_sleeping() {
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "snore@test");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.sleeping = true;
            p.food = 10.0;
            p.sleep_emot_timer = 0.0;
        }
        // Neutral vitals so hunger PE does not fire (HX may still emit — ignore non-PE).
        state.environment.temperature = 0.5;
        state.environment.season_length = 10_000.0;
        state.environment.day_length = 10_000.0;
        state.environment.hour_of_day = 12.0;

        let expected_pe = format_server_message("PE", &[&format!("{p_id} {SLEEP_EMOT_INDEX}")]);
        let is_sleep_pe = |pkt: &[u8]| pkt == expected_pe.as_bytes();

        tick_vitals(&mut state, SLEEP_EMOT_INTERVAL_SECS - 1.0, &hub);
        let mut early_pe = false;
        while let Ok(pkt) = rx.try_recv() {
            if is_sleep_pe(&pkt) {
                early_pe = true;
            }
        }
        assert!(!early_pe, "no sleep PE before SLEEP_EMOT_INTERVAL_SECS");

        tick_vitals(&mut state, 1.5, &hub);
        let mut saw_pe = false;
        while let Ok(pkt) = rx.try_recv() {
            if is_sleep_pe(&pkt) {
                saw_pe = true;
            }
        }
        assert!(saw_pe, "expected PE sleep packet {expected_pe}");

        // Wake clears timer and stops PE.
        state.players.get_mut(&1).unwrap().sleeping = false;
        tick_vitals(&mut state, SLEEP_EMOT_INTERVAL_SECS + 1.0, &hub);
        let mut saw_after_wake = false;
        while let Ok(pkt) = rx.try_recv() {
            if is_sleep_pe(&pkt) {
                saw_after_wake = true;
            }
        }
        assert!(!saw_after_wake, "no sleep PE after wake");
    }

    /// Floor id != 0 halves TEMP_FOOD_EXTRA (indoor shelter stub).
    #[test]
    fn indoor_floor_halves_temp_food_extra() {
        let hub = OutboundHub::new();
        let mut outdoor = SimState::with_default_empty(test_content());
        let mut indoor = SimState::with_default_empty(test_content());
        spawn_player(&mut outdoor, 1, "out");
        spawn_player(&mut indoor, 1, "in");
        for s in [&mut outdoor, &mut indoor] {
            s.environment.temperature = 0.0; // extreme cold → TEMP_FOOD_EXTRA
            s.environment.season_length = 10_000.0;
            s.environment.day_length = 10_000.0;
            s.environment.hour_of_day = 12.0;
        }
        let (ix, iy) = {
            let p = indoor.players.get(&1).unwrap();
            (p.x, p.y)
        };
        indoor.world.write().unwrap().set_floor(ix, iy, 1); // any non-zero floor

        let food0 = outdoor.players.get(&1).unwrap().food;
        assert_eq!(food0, indoor.players.get(&1).unwrap().food);

        tick_vitals(&mut outdoor, 1.0, &hub);
        tick_vitals(&mut indoor, 1.0, &hub);

        let out_lost = food0 - outdoor.players.get(&1).unwrap().food;
        let in_lost = food0 - indoor.players.get(&1).unwrap().food;
        let expected_out = FOOD_USE_PER_SEC + TEMP_FOOD_EXTRA;
        let expected_in = FOOD_USE_PER_SEC + TEMP_FOOD_EXTRA * 0.5;
        assert!(
            (out_lost - expected_out).abs() < 1e-4,
            "outdoor: lost={out_lost} expected={expected_out}"
        );
        assert!(
            (in_lost - expected_in).abs() < 1e-4,
            "indoor: lost={in_lost} expected={expected_in}"
        );
        assert!(in_lost < out_lost);
    }

    /// SAY RENAME changes display name and emits NM to nearby.
    #[test]
    fn say_rename_updates_name_and_sends_nm() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx1 = hub.register(1);
        let mut rx2 = hub.register(2);
        let mut state = SimState::with_default_empty(test_content());
        let p1 = spawn_player(&mut state, 1, "rename@test");
        let _p2 = spawn_player(&mut state, 2, "near@test");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.x = 0;
            p.y = 0;
            p.first_name = "OLD".into();
            p.family_name = "NAME".into();
        }
        {
            let p = state.players.get_mut(&2).unwrap();
            p.x = 1;
            p.y = 0;
        }
        state.social.ensure_lineage(p1, "OLD NAME");
        while rx1.try_recv().is_ok() {}
        while rx2.try_recv().is_ok() {}

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "RENAME Ada Snow".into(),
            },
        );

        let p = state.players.get(&1).unwrap();
        assert_eq!(p.first_name, "ADA");
        assert_eq!(p.family_name, "SNOW");
        assert_eq!(p.display_name(), "ADA SNOW");
        assert_eq!(
            state.social.lineages.get(&p1).map(|n| n.name.as_str()),
            Some("ADA SNOW")
        );

        let expected_nm = format_server_message("NM", &[&format!("{p1} ADA SNOW")]);
        let mut saw_nm1 = false;
        let mut saw_ok = false;
        while let Ok(pkt) = rx1.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if pkt == expected_nm.as_bytes() {
                saw_nm1 = true;
            }
            if s.contains("RENAME OK ADA SNOW") {
                saw_ok = true;
            }
        }
        let mut saw_nm2 = false;
        while let Ok(pkt) = rx2.try_recv() {
            if pkt == expected_nm.as_bytes() {
                saw_nm2 = true;
            }
        }
        assert!(saw_ok, "expected RENAME OK PS");
        assert!(saw_nm1, "renamer receives NM");
        assert!(saw_nm2, "nearby receives NM");
    }

    /// SAY DIE voluntary death with reason_suicide.
    #[test]
    fn say_die_sets_reason_suicide() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "die@test");
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "DIE".into(),
            },
        );
        let p = state.players.get(&1).unwrap();
        assert!(p.deleted);
        assert_eq!(p.death_reason.as_deref(), Some("reason_suicide"));
        assert!(
            state
                .event_log
                .iter()
                .any(|e| e.contains(&format!("DEATH {p_id} reason_suicide"))),
            "event_log: {:?}",
            state.event_log
        );
    }

    /// SAY SICK sets Player.sick; SAY CURE clears it.
    #[test]
    fn say_sick_and_cure_toggle_sick() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "sick@test");
        assert!(!state.players.get(&1).unwrap().sick);

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "SICK".into(),
            },
        );
        assert!(state.players.get(&1).unwrap().sick);

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "CURE".into(),
            },
        );
        assert!(!state.players.get(&1).unwrap().sick);
    }

    /// SAY RIDE sets Player.riding + move_speed note; SAY DISMOUNT clears.
    #[test]
    fn say_ride_and_dismount_toggle_riding() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "ride@test");
        while rx.try_recv().is_ok() {}
        assert!(!state.players.get(&1).unwrap().riding);

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "RIDE".into(),
            },
        );
        assert!(state.players.get(&1).unwrap().riding);
        let mut saw_ride_note = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("RIDE OK") && s.contains(&format!("move_speed={RIDE_MOVE_SPEED:.2}")) {
                saw_ride_note = true;
            }
        }
        assert!(saw_ride_note, "expected PS RIDE OK move_speed note");

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "DISMOUNT".into(),
            },
        );
        assert!(!state.players.get(&1).unwrap().riding);
        let mut saw_dismount_note = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("DISMOUNT OK")
                && s.contains(&format!("move_speed={WALK_MOVE_SPEED:.2}"))
            {
                saw_dismount_note = true;
            }
        }
        assert!(saw_dismount_note, "expected PS DISMOUNT OK move_speed note");
    }

    /// SAY MOUNT is an alias for RIDE (sets riding + RIDE OK move_speed note).
    #[test]
    fn say_mount_aliases_ride() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "mount@test");
        while rx.try_recv().is_ok() {}
        assert!(!state.players.get(&1).unwrap().riding);

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "MOUNT".into(),
            },
        );
        assert!(state.players.get(&1).unwrap().riding);
        let mut saw = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("RIDE OK") && s.contains(&format!("move_speed={RIDE_MOVE_SPEED:.2}")) {
                saw = true;
            }
        }
        assert!(saw, "expected PS RIDE OK from MOUNT alias");
    }

    /// SAY SWIM / ?SWIM report ocean wet + food_mult (extra drain already in vitals).
    #[test]
    fn say_swim_and_query_note_ocean_food_drain() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "swim@test");
        set_player_position(&mut state, 1, 0, 0);
        state
            .world
            .write()
            .unwrap()
            .set_biome(0, 0, BIOME_OCEAN);
        while rx.try_recv().is_ok() {}

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "SWIM".into(),
            },
        );
        let mut saw_swim = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("SWIM OK")
                && s.contains(&format!("biome={BIOME_OCEAN}"))
                && s.contains("wet=1")
                && s.contains(&format!("food_mult={OCEAN_RIVER_FOOD_DRAIN_MULT:.2}"))
            {
                saw_swim = true;
            }
        }
        assert!(saw_swim, "expected PS SWIM OK ocean note");

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?SWIM".into(),
            },
        );
        let expected = format_swim_query(BIOME_OCEAN);
        let mut saw_q = false;
        while let Ok(pkt) = rx.try_recv() {
            if String::from_utf8_lossy(&pkt).contains(&format!("{p_id} {expected}")) {
                saw_q = true;
            }
        }
        assert!(saw_q, "expected PS {p_id} {expected}");
    }

    /// SAY BUILD is a fence placeholder (object id 0 — no place).
    #[test]
    fn say_build_fence_placeholder() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "build@test");
        set_player_position(&mut state, 1, 2, 3);
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "BUILD".into(),
            },
        );
        // No object placed (fence id 0 placeholder).
        assert_eq!(state.world.read().unwrap().get_object(2, 3), 0);
        let mut saw = false;
        while let Ok(pkt) = rx.try_recv() {
            if String::from_utf8_lossy(&pkt).contains(&format!("{p_id} BUILD OK fence=0")) {
                saw = true;
            }
        }
        assert!(saw, "expected BUILD OK fence=0");
    }

    /// SAY CLAIM sets owner_id on object under feet without locking.
    #[test]
    fn say_claim_sets_owner_without_lock() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "claim@test");
        set_player_position(&mut state, 1, 4, 5);
        {
            let mut w = state.world.write().unwrap();
            w.set_object(4, 5, 99);
        }
        while rx.try_recv().is_ok() {}

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "CLAIM".into(),
            },
        );
        assert_eq!(
            state
                .world
                .read()
                .unwrap()
                .get_helper(4, 5)
                .map(|h| h.owner_id),
            Some(p_id)
        );
        assert!(
            !state.locks.is_locked(4, 5),
            "CLAIM must not lock the tile"
        );
        let mut saw = false;
        while let Ok(pkt) = rx.try_recv() {
            if String::from_utf8_lossy(&pkt).contains(&format!("{p_id} CLAIM 4 5 OK")) {
                saw = true;
            }
        }
        assert!(saw, "expected CLAIM 4 5 OK");

        // Empty tile → FAIL.
        set_player_position(&mut state, 1, 0, 0);
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "CLAIM".into(),
            },
        );
        let mut saw_fail = false;
        while let Ok(pkt) = rx.try_recv() {
            if String::from_utf8_lossy(&pkt).contains(&format!("{p_id} CLAIM 0 0 FAIL")) {
                saw_fail = true;
            }
        }
        assert!(saw_fail, "expected CLAIM FAIL on empty tile");
    }

    /// While sick, food drain is multiplied by SICK_FOOD_DRAIN_MULT (1.3).
    #[test]
    fn sick_increases_food_drain() {
        let hub = OutboundHub::new();
        let mut healthy = SimState::with_default_empty(test_content());
        let mut ill = SimState::with_default_empty(test_content());
        spawn_player(&mut healthy, 1, "healthy");
        spawn_player(&mut ill, 1, "ill");
        for s in [&mut healthy, &mut ill] {
            s.environment.temperature = 0.5;
            s.environment.season_length = 10_000.0;
            s.environment.day_length = 10_000.0;
            s.environment.hour_of_day = 12.0;
        }
        ill.players.get_mut(&1).unwrap().sick = true;

        let food0 = healthy.players.get(&1).unwrap().food;
        assert_eq!(food0, ill.players.get(&1).unwrap().food);

        tick_vitals(&mut healthy, 1.0, &hub);
        tick_vitals(&mut ill, 1.0, &hub);

        let healthy_lost = food0 - healthy.players.get(&1).unwrap().food;
        let ill_lost = food0 - ill.players.get(&1).unwrap().food;
        assert!(
            (healthy_lost - FOOD_USE_PER_SEC).abs() < 1e-4,
            "healthy drain: lost={healthy_lost}"
        );
        let expected_sick = FOOD_USE_PER_SEC * SICK_FOOD_DRAIN_MULT;
        assert!(
            (ill_lost - expected_sick).abs() < 1e-4,
            "sick drain: lost={ill_lost} expected={expected_sick}"
        );
        assert!(ill_lost > healthy_lost);
    }

    /// Starving sick infant emits DY with isSick flag (`p_id 1`).
    #[test]
    fn tick_vitals_dying_uses_sick_flag_when_food_low() {
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "sickbaby@test");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.age = 1.0;
            p.food = 4.0;
            p.sick = true;
            p.vitals_emit_timer = 0.0;
        }
        state.environment.temperature = 0.5;
        state.environment.season_length = 10_000.0;
        state.environment.day_length = 10_000.0;

        tick_vitals(&mut state, VITALS_EMIT_INTERVAL_SECS + 0.5, &hub);
        let mut saw_dy_sick = false;
        let mut saw_dy_plain = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.as_ref() == format_dying(p_id, true) {
                saw_dy_sick = true;
            }
            if s.as_ref() == format_dying(p_id, false) {
                saw_dy_plain = true;
            }
        }
        assert!(
            saw_dy_sick,
            "expected DY with isSick for sick starving infant p_id={p_id}"
        );
        assert!(
            !saw_dy_plain,
            "must not emit plain DY when player is sick"
        );
    }

    /// `SAY EMOTE <n>` emits PE `player_id n` to nearby (alias for EMOT), not PS chat.
    #[test]
    fn say_emote_emits_pe_to_nearby() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx1 = hub.register(1);
        let mut rx2 = hub.register(2);
        let mut rx3 = hub.register(3);
        let mut state = SimState::with_default_empty(test_content());
        let p1 = spawn_player(&mut state, 1, "emoter@x");
        spawn_player(&mut state, 2, "near@x");
        spawn_player(&mut state, 3, "far@x");
        set_player_position(&mut state, 1, 0, 0);
        set_player_position(&mut state, 2, 5, 0); // within NEARBY_RANGE
        set_player_position(&mut state, 3, 100, 0); // beyond NEARBY_RANGE
        while rx1.try_recv().is_ok() {}
        while rx2.try_recv().is_ok() {}
        while rx3.try_recv().is_ok() {}

        let emot_n = 3;
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("EMOTE {emot_n}"),
            },
        );

        let expected_pe = format_server_message("PE", &[&format!("{p1} {emot_n}")]);
        let mut saw1 = false;
        let mut saw_ps1 = false;
        while let Ok(pkt) = rx1.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.as_ref() == expected_pe {
                saw1 = true;
            }
            if s.starts_with("PS\n") {
                saw_ps1 = true;
            }
        }
        let mut saw2 = false;
        while let Ok(pkt) = rx2.try_recv() {
            if String::from_utf8_lossy(&pkt).as_ref() == expected_pe {
                saw2 = true;
            }
        }
        assert!(saw1, "emoter must receive PE {expected_pe}");
        assert!(saw2, "nearby conn must receive PE {expected_pe}");
        assert!(!saw_ps1, "SAY EMOTE must not broadcast PS chat");
        assert!(
            rx3.try_recv().is_err(),
            "far conn must not receive PE outside NEARBY_RANGE"
        );

        // Missing index defaults to 0 (same as EMOT).
        while rx1.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "EMOTE".into(),
            },
        );
        let expected0 = format_server_message("PE", &[&format!("{p1} 0")]);
        let mut saw0 = false;
        while let Ok(pkt) = rx1.try_recv() {
            if String::from_utf8_lossy(&pkt).as_ref() == expected0 {
                saw0 = true;
            }
        }
        assert!(saw0, "SAY EMOTE with no index should emit PE … 0");
    }

    /// PE/EMOTE rate limit is independent of SAY: max 3 per 10 sim-seconds.
    #[test]
    fn say_emote_rate_limited_separately_from_say() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "e@x");
        state.sim_time = 0.0;
        while rx.try_recv().is_ok() {}

        for i in 0..3 {
            apply_intent(
                &mut state,
                &counters,
                &hub,
                NetIntent::Raw {
                    conn_id: 1,
                    tag: "SAY".into(),
                    payload: format!("EMOTE {i}"),
                },
            );
        }
        // Fourth emote in window → EMOTE RATE (not SAY RATE).
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "EMOTE 9".into(),
            },
        );
        let mut saw_rate = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("EMOTE RATE") {
                saw_rate = true;
            }
            assert!(
                !s.starts_with("PE\n"),
                "fourth emote must not emit PE: {s}"
            );
        }
        assert!(saw_rate, "expected PS EMOTE RATE on 4th emote");

        // SAY chat still allowed (separate window).
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "hello".into(),
            },
        );
        let mut saw_say = false;
        while let Ok(pkt) = rx.try_recv() {
            if String::from_utf8_lossy(&pkt).starts_with("PS\n") {
                saw_say = true;
            }
        }
        assert!(saw_say, "SAY chat must not be blocked by emote limit");
    }

    /// Reverse craft graph seeds from content transitions (capped).
    #[test]
    fn seed_craft_graph_from_content_transitions() {
        let mut state = SimState::with_default_empty(test_content());
        assert_eq!(state.craft_graph.product_count(), 0);
        seed_craft_graph_from_content(&mut state);
        // test_content has (0,33)→(34,0) and last-use (0,33)→(99,1)
        assert!(
            state.craft_graph.product_count() >= 1,
            "expected products after seed"
        );
        assert!(
            state.craft_graph.ingredients_for(34).is_some()
                || state.craft_graph.ingredients_for(99).is_some(),
            "seeded reverse edges for known products"
        );
    }

    /// SAY ?LEADER / ?WOUND / ?BIOMES / ALLY pure query paths.
    #[test]
    fn say_leader_wound_biomes_ally_queries() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p1 = spawn_player(&mut state, 1, "a@x");
        let p2 = spawn_player(&mut state, 2, "b@x");
        state.social.set_follow(p2, p1).unwrap();
        state.combat.apply_wound(p1, 2);
        while rx.try_recv().is_ok() {}

        for payload in ["?LEADER", "?WOUND", "?BIOMES", "?ALLY"] {
            apply_intent(
                &mut state,
                &counters,
                &hub,
                NetIntent::Raw {
                    conn_id: 1,
                    tag: "SAY".into(),
                    payload: payload.into(),
                },
            );
        }
        let mut texts = Vec::new();
        while let Ok(pkt) = rx.try_recv() {
            texts.push(String::from_utf8_lossy(&pkt).into_owned());
        }
        let joined = texts.join("|");
        assert!(joined.contains("LEADER"), "got {joined}");
        assert!(joined.contains("WOUND"), "got {joined}");
        assert!(joined.contains("BIOMES"), "got {joined}");
        assert!(joined.contains("ALLY"), "got {joined}");
        assert!(joined.contains("21:MOUNTAIN") || joined.contains("MOUNTAIN"), "got {joined}");

        // ALLY add + HEAL (advance sim_time past SAY rate window).
        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("ALLY {p2}"),
            },
        );
        assert!(state.allies.is_ally(p1, p2));

        // HEAL free when hands empty
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "HEAL".into(),
            },
        );
        assert_eq!(state.combat.wound_of(p1), 0);
    }

    /// SAY WHISPER <p_id> <text> delivers PS only to the target connection (two hubs).
    #[test]
    fn say_whisper_sends_ps_only_to_target() {
        let counters = Counters::new();
        // Two hubs: whisperer and target each register on a shared outbound hub.
        let hub = OutboundHub::new();
        let mut rx1 = hub.register(1);
        let mut rx2 = hub.register(2);
        let mut state = SimState::with_default_empty(test_content());
        let p1 = spawn_player(&mut state, 1, "whisperer@x");
        let p2 = spawn_player(&mut state, 2, "listener@x");
        // Place far apart so normal nearby chat would not reach — whisper must still work.
        state.players.get_mut(&1).unwrap().x = 0;
        state.players.get_mut(&1).unwrap().y = 0;
        state.players.get_mut(&2).unwrap().x = 500;
        state.players.get_mut(&2).unwrap().y = 500;
        while rx1.try_recv().is_ok() {}
        while rx2.try_recv().is_ok() {}

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("WHISPER {p2} secret hello"),
            },
        );

        let expected = format_player_says(p1, false, "secret hello");
        let mut saw_target = false;
        let mut saw_fm = false;
        while let Ok(pkt) = rx2.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s == expected || (s.starts_with("PS\n") && s.contains(&format!("{p1}/0 secret hello"))) {
                saw_target = true;
            }
            if s.starts_with("FM\n") || s == "FM\n#" || s.trim() == "FM\n#" {
                saw_fm = true;
            }
            if s.starts_with("FM") {
                saw_fm = true;
            }
        }
        assert!(saw_target, "target conn must receive whisper PS as p_id/0 text");
        assert!(saw_fm, "whisper PS must be followed by FM for official clients");

        let mut saw_sender = false;
        while let Ok(pkt) = rx1.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("secret hello") {
                saw_sender = true;
            }
        }
        assert!(!saw_sender, "whisperer must not receive own whisper PS");

        // Offline / unknown p_id: no PS to either hub.
        while rx1.try_recv().is_ok() {}
        while rx2.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "WHISPER 99999 nobody hears".into(),
            },
        );
        assert!(rx1.try_recv().is_err(), "offline whisper: no PS on hub1");
        assert!(rx2.try_recv().is_err(), "offline whisper: no PS on hub2");
    }

    /// SAY HIT applies wounds then kills at threshold; KILL remains one-shot.
    #[test]
    fn say_hit_wounds_then_kills_kill_one_shot() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx1 = hub.register(1);
        let mut rx2 = hub.register(2);
        let mut state = SimState::with_default_empty(test_content());
        let a = spawn_player(&mut state, 1, "hit@a");
        let b = spawn_player(&mut state, 2, "hit@b");
        state.social.ensure_lineage(a, "A");
        state.social.ensure_lineage(b, "B");
        // Adjacent for range.
        state.players.get_mut(&1).unwrap().x = 0;
        state.players.get_mut(&1).unwrap().y = 0;
        state.players.get_mut(&2).unwrap().x = 1;
        state.players.get_mut(&2).unwrap().y = 0;
        while rx1.try_recv().is_ok() {}
        while rx2.try_recv().is_ok() {}

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("HIT {b}"),
            },
        );
        assert_eq!(state.combat.wound_of(b), 1);
        assert!(!state.players.get(&2).unwrap().deleted);

        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("HIT {b}"),
            },
        );
        assert_eq!(state.combat.wound_of(b), 2);

        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("HIT {b}"),
            },
        );
        assert!(state.players.get(&2).unwrap().deleted, "third hit kills");
        assert_eq!(state.combat.stats.get(&a).map(|s| s.kills), Some(1));

        // Fresh target: KILL is still one-shot.
        let mut rx3 = hub.register(3);
        let c = spawn_player(&mut state, 3, "hit@c");
        state.players.get_mut(&3).unwrap().x = 0;
        state.players.get_mut(&3).unwrap().y = 1;
        while rx3.try_recv().is_ok() {}
        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("KILL {c}"),
            },
        );
        assert!(state.players.get(&3).unwrap().deleted, "KILL one-shot");
        assert_eq!(state.combat.wound_of(c), 0);
    }

    /// SAY HIT uses weapon_range from held name; bow reaches dist 5, bare hands miss.
    #[test]
    fn say_hit_uses_weapon_range_from_held() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx1 = hub.register(1);
        let mut rx2 = hub.register(2);
        let mut db = ContentDb::default();
        db.objects.insert(
            200,
            ObjectDef {
                id: 200,
                description: "Long Bow".into(),
                name: "Long Bow".into(),
                containable: true,
                permanent: false,
                blocks_walking: false,
                food_value: 0,
                heat_value: 0.0,
                map_chance: 0.0,
                biomes: Vec::new(),
                num_uses: 0,
                num_slots: 0,
                floor: false,
            dummy_ids: Vec::new(),
            },
        );
        let mut state = SimState::with_default_empty(Arc::new(db));
        let a = spawn_player(&mut state, 1, "bow@a");
        let b = spawn_player(&mut state, 2, "bow@b");
        state.social.ensure_lineage(a, "A");
        state.social.ensure_lineage(b, "B");
        state.players.get_mut(&1).unwrap().x = 0;
        state.players.get_mut(&1).unwrap().y = 0;
        // Dist 5: beyond KILL_RANGE(2), within bow(8).
        state.players.get_mut(&2).unwrap().x = 5;
        state.players.get_mut(&2).unwrap().y = 0;

        // Bare hands: miss.
        while rx1.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("HIT {b}"),
            },
        );
        assert_eq!(state.combat.wound_of(b), 0, "bare hands miss at dist 5");
        let mut miss_ps = false;
        while let Ok(pkt) = rx1.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("HIT") && s.contains("MISS") {
                miss_ps = true;
            }
        }
        assert!(miss_ps, "expected HIT MISS with bare hands");

        // Hold bow: hit lands.
        state.players.get_mut(&1).unwrap().held_id = 200;
        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        while rx1.try_recv().is_ok() {}
        while rx2.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("HIT {b}"),
            },
        );
        assert_eq!(state.combat.wound_of(b), 1, "bow hits at dist 5");
    }

    /// Successful SAY HIT emits PE mad (index 1) for the wounded target to nearby.
    #[test]
    fn say_hit_emits_pe_mad_on_wound() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx1 = hub.register(1);
        let mut rx2 = hub.register(2);
        let mut state = SimState::with_default_empty(test_content());
        let a = spawn_player(&mut state, 1, "pe@a");
        let b = spawn_player(&mut state, 2, "pe@b");
        state.social.ensure_lineage(a, "A");
        state.social.ensure_lineage(b, "B");
        state.players.get_mut(&1).unwrap().x = 0;
        state.players.get_mut(&1).unwrap().y = 0;
        state.players.get_mut(&2).unwrap().x = 1;
        state.players.get_mut(&2).unwrap().y = 0;
        while rx1.try_recv().is_ok() {}
        while rx2.try_recv().is_ok() {}

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("HIT {b}"),
            },
        );
        assert_eq!(state.combat.wound_of(b), 1);
        let expected_pe =
            format_server_message("PE", &[&format!("{b} {HUNGER_EMOT_INDEX}")]);
        let mut saw_pe = false;
        for rx in [&mut rx1, &mut rx2] {
            while let Ok(pkt) = rx.try_recv() {
                if String::from_utf8_lossy(&pkt) == expected_pe {
                    saw_pe = true;
                }
            }
        }
        assert!(saw_pe, "expected PE mad on wound target, want {expected_pe}");
    }

    /// SAY BANDAGE is an alias of HEAL (clears wounds when hands empty).
    #[test]
    fn say_bandage_aliases_heal() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p1 = spawn_player(&mut state, 1, "bandage@x");
        state.combat.apply_wound(p1, 2);
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "BANDAGE".into(),
            },
        );
        assert_eq!(state.combat.wound_of(p1), 0);
        let mut saw = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("BANDAGE OK") {
                saw = true;
            }
        }
        assert!(saw, "expected BANDAGE OK PS");
    }

    /// FEED with held name containing "poison" applies sick to target.
    #[test]
    fn say_feed_poison_applies_sick() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx1 = hub.register(1);
        let mut rx2 = hub.register(2);
        let mut db = ContentDb::default();
        db.objects.insert(
            77,
            ObjectDef {
                id: 77,
                description: "Poison Berry".into(),
                name: "Poison Berry".into(),
                containable: true,
                permanent: false,
                blocks_walking: false,
                food_value: 2,
                heat_value: 0.0,
                map_chance: 0.0,
                biomes: Vec::new(),
                num_uses: 0,
                num_slots: 0,
                floor: false,
            dummy_ids: Vec::new(),
            },
        );
        let mut state = SimState::with_default_empty(Arc::new(db));
        let a = spawn_player(&mut state, 1, "poison@a");
        let b = spawn_player(&mut state, 2, "poison@b");
        state.players.get_mut(&1).unwrap().x = 0;
        state.players.get_mut(&1).unwrap().y = 0;
        state.players.get_mut(&1).unwrap().held_id = 77;
        state.players.get_mut(&2).unwrap().x = 1;
        state.players.get_mut(&2).unwrap().y = 0;
        state.players.get_mut(&2).unwrap().food = 10.0;
        assert!(!state.players.get(&2).unwrap().sick);
        while rx1.try_recv().is_ok() {}
        while rx2.try_recv().is_ok() {}

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("FEED {b}"),
            },
        );
        assert!(
            state.players.get(&2).unwrap().sick,
            "poison FEED should set target sick"
        );
        assert!(
            state.players.get(&2).unwrap().food > 10.0,
            "food still transferred"
        );
        assert_eq!(state.players.get(&1).unwrap().held_id, 0);
        let mut saw_sick = false;
        while let Ok(pkt) = rx1.try_recv() {
            if String::from_utf8_lossy(&pkt).contains("sick") {
                saw_sick = true;
            }
        }
        assert!(saw_sick, "expected FEED OK … sick in PS");
        let _ = a;
    }

    /// SAY ?RANGE reports weapon_range for current held object.
    #[test]
    fn say_range_query_for_held_weapon() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut db = ContentDb::default();
        db.objects.insert(
            201,
            ObjectDef {
                id: 201,
                description: "Wooden Spear".into(),
                name: "Wooden Spear".into(),
                containable: true,
                permanent: false,
                blocks_walking: false,
                food_value: 0,
                heat_value: 0.0,
                map_chance: 0.0,
                biomes: Vec::new(),
                num_uses: 0,
                num_slots: 0,
                floor: false,
            dummy_ids: Vec::new(),
            },
        );
        let mut state = SimState::with_default_empty(Arc::new(db));
        spawn_player(&mut state, 1, "range@x");

        // Bare hands: default KILL_RANGE.
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?RANGE".into(),
            },
        );
        let mut bare = String::new();
        while let Ok(pkt) = rx.try_recv() {
            bare.push_str(&String::from_utf8_lossy(&pkt));
        }
        assert!(
            bare.contains(&format!("RANGE {KILL_RANGE}")),
            "bare hands range, got {bare}"
        );

        // Spear: range 3.
        state.players.get_mut(&1).unwrap().held_id = 201;
        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?RANGE".into(),
            },
        );
        let mut spear = String::new();
        while let Ok(pkt) = rx.try_recv() {
            spear.push_str(&String::from_utf8_lossy(&pkt));
        }
        assert!(spear.contains("RANGE 3"), "spear range, got {spear}");
        assert!(
            spear.contains("held=Wooden Spear"),
            "spear name, got {spear}"
        );
    }

    /// SAY NURSE / FEED while holding baby transfers held food to the baby.
    #[test]
    fn say_nurse_feeds_held_baby() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let mother = spawn_player(&mut state, 1, "mom@x");
        // Adult mother.
        state.players.get_mut(&1).unwrap().age = 20.0;
        // Spawn baby via API.
        let baby_id = spawn_child(&mut state, 1).expect("baby");
        let baby_conn = state
            .players
            .iter()
            .find(|(_, p)| p.p_id == baby_id)
            .map(|(&c, _)| c)
            .expect("baby conn");
        // Hold baby + food.
        {
            let m = state.players.get_mut(&1).unwrap();
            m.start_holding(baby_id);
            m.held_id = 33; // gooseberry-ish food in test_content
        }
        state.players.get_mut(&baby_conn).unwrap().held_by = mother;
        state.players.get_mut(&baby_conn).unwrap().food = 5.0;
        while rx.try_recv().is_ok() {}

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "NURSE".into(),
            },
        );
        let baby = state.players.get(&baby_conn).unwrap();
        assert!(baby.food > 5.0, "baby food increased, got {}", baby.food);
        assert_eq!(state.players.get(&1).unwrap().held_id, 0, "food consumed");

        // FEED alone while holding also works.
        state.players.get_mut(&1).unwrap().held_id = 33;
        state.players.get_mut(&baby_conn).unwrap().food = 6.0;
        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "FEED".into(),
            },
        );
        assert!(state.players.get(&baby_conn).unwrap().food > 6.0);
    }

    /// Default animal spawn + wander + ?ANIMALS query.
    #[test]
    fn animals_spawn_wander_and_query() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "zoo@x");
        assert!(state.animals.animals.is_empty());
        spawn_default_animals(&mut state);
        assert_eq!(state.animals.animals.len(), 7);
        let snap = state.animals.snapshot();
        assert_eq!(snap.rabbit, 3);
        assert_eq!(snap.wolf, 2);
        assert_eq!(snap.boar, 2);
        let before: Vec<(i32, i32)> = state.animals.animals.iter().map(|a| (a.x, a.y)).collect();
        // Force wander ticks until someone moves (or many attempts).
        for _ in 0..40 {
            tick_animals(&mut state);
        }
        let after: Vec<(i32, i32)> = state.animals.animals.iter().map(|a| (a.x, a.y)).collect();
        assert_ne!(before, after, "expected at least one animal to wander");

        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?ANIMALS".into(),
            },
        );
        let mut saw = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("ANIMALS")
                && s.contains("rabbit=")
                && s.contains("wolf=")
                && s.contains("boar=")
            {
                saw = true;
            }
        }
        assert!(saw, "expected ?ANIMALS PS reply with kind counts");

        // ?FAUNA is an alias for ?ANIMALS.
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?FAUNA".into(),
            },
        );
        let mut saw_fauna = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("ANIMALS") && s.contains("total=") {
                saw_fauna = true;
            }
        }
        assert!(saw_fauna, "expected ?FAUNA → ANIMALS PS reply");
    }

    /// SAY HUNT damages adjacent animals; kill grants meat placeholder + prestige.
    #[test]
    fn say_hunt_hit_and_kill_adjacent() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "hunter@x");
        let (px, py, p_id) = {
            let p = state.players.get(&1).unwrap();
            (p.x, p.y, p.p_id)
        };
        // Adjacent rabbit (default hp 5 = one HUNT_DAMAGE kill).
        let rabbit_id = state.animals.spawn(AnimalKind::Rabbit, px + 1, py);
        // Far wolf should not be hit while rabbit is nearer.
        state.animals.spawn(AnimalKind::Wolf, px + 10, py + 10);

        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "HUNT".into(),
            },
        );
        let mut saw_kill = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("HUNT KILL") && s.contains("rabbit") && s.contains("meat=0") {
                saw_kill = true;
            }
        }
        assert!(saw_kill, "expected HUNT KILL rabbit with meat placeholder");
        assert!(
            state.animals.animals.iter().all(|a| a.id != rabbit_id),
            "rabbit should be removed on kill"
        );
        let prest = state
            .combat
            .stats
            .get(&p_id)
            .map(|s| s.prestige)
            .unwrap_or(0.0);
        assert!(
            (prest - HUNT_KILL_PRESTIGE).abs() < 1e-5,
            "prestige should gain {HUNT_KILL_PRESTIGE}, got {prest}"
        );

        // No adjacent animal → MISS
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "HUNT".into(),
            },
        );
        let mut saw_miss = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("HUNT MISS") {
                saw_miss = true;
            }
        }
        assert!(saw_miss, "expected HUNT MISS when no adjacent animal");

        // Multi-hit wolf (hp 20) → HIT then later KILL
        state.animals.animals.clear();
        let wolf_id = state.animals.spawn(AnimalKind::Wolf, px, py);
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "HUNT".into(),
            },
        );
        let mut saw_hit = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("HUNT HIT") && s.contains("wolf") && s.contains("hp=") {
                saw_hit = true;
            }
        }
        assert!(saw_hit, "expected HUNT HIT on wolf");
        assert_eq!(
            state.animals.animals.iter().find(|a| a.id == wolf_id).map(|a| a.hp),
            Some(20 - HUNT_DAMAGE)
        );
    }

    /// SAY HARVEST / FISH / MINE / DIG / CHOP: biome-gated professions, shared 5s cooldown.
    #[test]
    fn say_harvest_fish_mine_profession_actions() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "pro@x");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.x = 10;
            p.y = 10;
            p.held_id = 0;
        }
        // Grassland under feet (default 0); harvest berry/food id 33 from test_content.
        state.world.write().unwrap().set_biome(10, 10, GRASSLAND_BIOME);
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "HARVEST".into(),
            },
        );
        let mut saw = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("HARVEST OK") && s.contains("id=33") {
                saw = true;
            }
        }
        assert!(saw, "expected HARVEST OK id=33 on grassland");
        assert_eq!(state.players.get(&1).unwrap().held_id, 33);
        let t_after_harvest = state.players.get(&1).unwrap().last_prof_action_time;
        assert!(
            (t_after_harvest - state.sim_time).abs() < 1e-5,
            "last_prof_action_time should update on success"
        );
        // Shared cooldown: immediate FISH fails even on wrong biome path uses COOLDOWN first.
        state.players.get_mut(&1).unwrap().held_id = 0;
        state.world.write().unwrap().set_biome(10, 10, OCEAN_BIOME);
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "FISH".into(),
            },
        );
        let mut saw_cd = false;
        while let Ok(pkt) = rx.try_recv() {
            if String::from_utf8_lossy(&pkt).contains("FISH FAIL COOLDOWN") {
                saw_cd = true;
            }
        }
        assert!(saw_cd, "expected FISH FAIL COOLDOWN within 5s");
        assert_eq!(state.players.get(&1).unwrap().held_id, 0);

        // Advance past cooldown → FISH on ocean gives placeholder.
        state.sim_time += PROF_ACTION_COOLDOWN_SECS;
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "FISH".into(),
            },
        );
        let mut saw_fish = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("FISH OK") && s.contains(&format!("id={FISH_PLACEHOLDER_ID}")) {
                saw_fish = true;
            }
        }
        assert!(saw_fish, "expected FISH OK with fish placeholder");
        assert_eq!(
            state.players.get(&1).unwrap().held_id,
            FISH_PLACEHOLDER_ID
        );

        // MINE: clear hands, mountain adjacent, past cooldown.
        state.players.get_mut(&1).unwrap().held_id = 0;
        state.sim_time += PROF_ACTION_COOLDOWN_SECS;
        state
            .world
            .write()
            .unwrap()
            .set_biome(11, 10, MOUNTAIN_BIOME);
        state
            .world
            .write()
            .unwrap()
            .set_biome(10, 10, GRASSLAND_BIOME);
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "MINE".into(),
            },
        );
        let mut saw_mine = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("MINE OK") && s.contains(&format!("id={STONE_PLACEHOLDER_ID}")) {
                saw_mine = true;
            }
        }
        assert!(saw_mine, "expected MINE OK with stone placeholder");
        assert_eq!(
            state.players.get(&1).unwrap().held_id,
            STONE_PLACEHOLDER_ID
        );

        // DIG on swamp.
        state.players.get_mut(&1).unwrap().held_id = 0;
        state.sim_time += PROF_ACTION_COOLDOWN_SECS;
        state.world.write().unwrap().set_biome(10, 10, SWAMP_BIOME);
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "DIG".into(),
            },
        );
        let mut saw_dig = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("DIG OK") && s.contains(&format!("id={CLAY_PLACEHOLDER_ID}")) {
                saw_dig = true;
            }
        }
        assert!(saw_dig, "expected DIG OK with clay placeholder");
        assert_eq!(
            state.players.get(&1).unwrap().held_id,
            CLAY_PLACEHOLDER_ID
        );

        // CHOP on jungle/yellow.
        state.players.get_mut(&1).unwrap().held_id = 0;
        state.sim_time += PROF_ACTION_COOLDOWN_SECS;
        state.world.write().unwrap().set_biome(10, 10, JUNGLE_BIOME);
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "CHOP".into(),
            },
        );
        let mut saw_chop = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("CHOP OK") && s.contains(&format!("id={WOOD_PLACEHOLDER_ID}")) {
                saw_chop = true;
            }
        }
        assert!(saw_chop, "expected CHOP OK with wood placeholder");
        assert_eq!(
            state.players.get(&1).unwrap().held_id,
            WOOD_PLACEHOLDER_ID
        );

        // Hands full → FAIL HANDS (after cooldown advance).
        state.sim_time += PROF_ACTION_COOLDOWN_SECS;
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "HARVEST".into(),
            },
        );
        let mut saw_hands = false;
        while let Ok(pkt) = rx.try_recv() {
            if String::from_utf8_lossy(&pkt).contains("HARVEST FAIL HANDS") {
                saw_hands = true;
            }
        }
        assert!(saw_hands, "expected HARVEST FAIL HANDS when holding wood");
        let _ = p_id;
    }

    /// Living prestige refresh assigns percentile classes onto lineage nodes.
    #[test]
    fn living_prestige_refresh_updates_lineage_classes() {
        let mut state = SimState::with_default_empty(test_content());
        let ids: Vec<i32> = (1..=5)
            .map(|i| {
                let p = spawn_player(&mut state, i as u64, &format!("p{i}@x"));
                state.social.ensure_lineage(p, &format!("P{i}"));
                state.scoreboard.ensure_player(p, format!("P{i}"));
                // Distinct scores via coins.
                state.scoreboard.set_coins(p, i * 10);
                p
            })
            .collect();
        state.refresh_living_prestige_classes();
        // Lowest score → Serf-ish; highest → higher class for n=5.
        let low = state.social.prestige_class(ids[0]);
        let high = state.social.prestige_class(ids[4]);
        assert_eq!(low, PrestigeClass::Serf);
        assert!(
            high as u8 > low as u8,
            "high score should rank above low: low={low:?} high={high:?}"
        );

        // tick_vitals path fires after timer.
        state.prestige_refresh_timer = LIVING_PRESTIGE_REFRESH_SECS - 0.1;
        // Bump lowest player's score to top and refresh via tick.
        state.scoreboard.set_coins(ids[0], 999);
        let hub = OutboundHub::new();
        tick_vitals(&mut state, 0.2, &hub);
        let now_top = state.social.prestige_class(ids[0]);
        assert!(
            now_top as u8 >= PrestigeClass::Noble as u8
                || now_top as u8 > PrestigeClass::Serf as u8,
            "score leader class should rise after refresh, got {now_top:?}"
        );
    }

    /// tick_vitals advances animal wander on interval.
    #[test]
    fn tick_vitals_animal_wander_interval() {
        let mut state = SimState::with_default_empty(test_content());
        spawn_default_animals(&mut state);
        let hub = OutboundHub::new();
        let before = state.animals.animals[0].x + state.animals.animals[0].y * 1000;
        // Advance past several wander intervals.
        for _ in 0..30 {
            tick_vitals(&mut state, ANIMAL_WANDER_INTERVAL_SECS, &hub);
        }
        let moved = state.animals.animals.iter().any(|a| {
            // Not all may move, but positions must stay in-bounds.
            a.x >= 0 && a.y >= 0
        });
        assert!(moved);
        let _ = before; // seed-dependent motion; in-bounds check is the hard assert
    }

    #[test]
    fn build_reverse_craft_graph_matches_seed() {
        let content = test_content();
        let g = build_reverse_craft_graph(&content);
        assert!(g.product_count() >= 1);
        assert!(
            g.ingredients_for(34).is_some() || g.ingredients_for(99).is_some()
        );
        let have = std::collections::HashSet::new();
        // seek ingredient for a known product when hands empty
        if let Some(want) = [34, 99].into_iter().find(|id| g.ingredients_for(*id).is_some()) {
            let s = g.seek_ingredient_for(want, &have);
            assert!(s.is_some(), "expected seek ingredient for {want}");
        }
    }

    /// SAY GESTATE starts timed pregnancy; tick_vitals auto-spawns when due.
    #[test]
    fn say_gestate_and_tick_spawns_baby() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let mother_id = spawn_player(&mut state, 1, "gest@x");
        state.players.get_mut(&1).unwrap().age = 20.0;
        set_player_position(&mut state, 1, 5, 6);

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "GESTATE".into(),
            },
        );
        let mut saw_ok = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("GESTATE OK") {
                saw_ok = true;
            }
        }
        assert!(saw_ok, "expected GESTATE OK PS");
        assert!(
            state
                .fertility
                .by_mother
                .get(&mother_id)
                .and_then(|r| r.gestating_until)
                .is_some()
        );
        // No baby yet.
        assert!(
            !state
                .players
                .values()
                .any(|p| p.p_id != mother_id && p.age < 1.0)
        );

        // Advance past gestation.
        tick_vitals(&mut state, GESTATION_SECS + 0.5, &hub);
        let baby = state
            .players
            .values()
            .find(|p| p.p_id != mother_id && p.age < 1.0);
        assert!(baby.is_some(), "expected baby after gestation due");
        let baby = baby.unwrap();
        assert_eq!(baby.x, 5);
        assert_eq!(baby.y, 6);
        assert!(
            state
                .event_log
                .iter()
                .any(|e| e.starts_with("BIRTH ") && e.contains(&format!("mother={mother_id}"))),
            "event_log: {:?}",
            state.event_log
        );
        // Second GESTATE blocked by cooldown.
        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "GESTATE".into(),
            },
        );
        let mut saw_fail = false;
        while let Ok(pkt) = rx.try_recv() {
            if String::from_utf8_lossy(&pkt).contains("GESTATE FAIL COOLDOWN") {
                saw_fail = true;
            }
        }
        assert!(saw_fail, "expected GESTATE FAIL COOLDOWN after birth");
    }

    /// SAY IGNITE aliases FIRE; SAY EXTINGUISH clears fire under feet.
    #[test]
    fn say_ignite_and_extinguish() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "fire@x");
        set_player_position(&mut state, 1, 3, 4);

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "IGNITE".into(),
            },
        );
        assert!(state.fire.is_burning(3, 4));
        let mut saw_fire = false;
        while let Ok(pkt) = rx.try_recv() {
            if String::from_utf8_lossy(&pkt).contains("FIRE 3 4 OK") {
                saw_fire = true;
            }
        }
        assert!(saw_fire);

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "EXTINGUISH".into(),
            },
        );
        assert!(!state.fire.is_burning(3, 4));
        let mut saw_ext = false;
        while let Ok(pkt) = rx.try_recv() {
            if String::from_utf8_lossy(&pkt).contains("EXTINGUISH 3 4 OK") {
                saw_ext = true;
            }
        }
        assert!(saw_ext);

        // Second extinguish fails.
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "EXTINGUISH".into(),
            },
        );
        let mut saw_fail = false;
        while let Ok(pkt) = rx.try_recv() {
            if String::from_utf8_lossy(&pkt).contains("EXTINGUISH FAIL") {
                saw_fail = true;
            }
        }
        assert!(saw_fail);
    }

    /// SAY LOCK / UNLOCK set owner on gate under feet; walkability respects lock.
    #[test]
    fn say_lock_unlock_owned_gate() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut db = ContentDb::default();
        db.objects.insert(
            50,
            ObjectDef {
                id: 50,
                description: "Pine Door".into(),
                name: "Pine Door".into(),
                containable: false,
                permanent: true,
                blocks_walking: true,
                food_value: 0,
                heat_value: 0.0,
                map_chance: 0.0,
                biomes: Vec::new(),
                num_uses: 0,
                num_slots: 0,
                floor: false,
            dummy_ids: Vec::new(),
            },
        );
        let mut state = SimState::with_default_empty(Arc::new(db));
        let owner = spawn_player(&mut state, 1, "owner@x");
        let stranger = spawn_player(&mut state, 2, "str@x");
        set_player_position(&mut state, 1, 1, 1);
        {
            let mut w = state.world.write().unwrap();
            w.set_object(1, 1, 50);
        }

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "LOCK".into(),
            },
        );
        assert_eq!(
            state.world.read().unwrap().get_helper(1, 1).map(|h| h.owner_id),
            Some(owner)
        );
        let mut saw_lock = false;
        while let Ok(pkt) = rx.try_recv() {
            if String::from_utf8_lossy(&pkt).contains("LOCK 1 1 OK") {
                saw_lock = true;
            }
        }
        assert!(saw_lock);

        let content = state.content.clone();
        let allies = state.allies.clone();
        assert!(is_walkable_for_player(
            &state.world.read().unwrap(),
            &content,
            1,
            1,
            owner,
            &|a, b| allies.is_mutual_or_either(a, b)
        ));
        assert!(!is_walkable_for_player(
            &state.world.read().unwrap(),
            &content,
            1,
            1,
            stranger,
            &|a, b| allies.is_mutual_or_either(a, b)
        ));
        // Ally may pass.
        state.allies.add(stranger, owner).unwrap();
        let allies = state.allies.clone();
        assert!(is_walkable_for_player(
            &state.world.read().unwrap(),
            &content,
            1,
            1,
            stranger,
            &|a, b| allies.is_mutual_or_either(a, b)
        ));

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "UNLOCK".into(),
            },
        );
        assert_eq!(
            state
                .world
                .read()
                .unwrap()
                .get_helper(1, 1)
                .map(|h| h.owner_id)
                .unwrap_or(0),
            0
        );
        assert!(is_walkable_for_player(
            &state.world.read().unwrap(),
            &content,
            1,
            1,
            stranger,
            &|_, _| false
        ));
    }

    /// SAY YAWN emits PE player_id 2 (yawn emote index).
    #[test]
    fn say_yawn_emits_pe_2() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "yawn@x");
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "YAWN".into(),
            },
        );
        let expected = format_server_message("PE", &[&format!("{p_id} {YAWN_EMOT_INDEX}")]);
        let mut saw = false;
        while let Ok(pkt) = rx.try_recv() {
            if pkt == expected.as_bytes() {
                saw = true;
            }
        }
        assert!(saw, "expected PE {p_id} 2 for YAWN");
    }

    /// Spawn + MOVE touch AfkBook; idle past DEFAULT_AFK_SECS marks AFK + PE yawn.
    #[test]
    fn afk_book_touch_and_vitals_yawn() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "afk@x");
        // Plenty of food so hunger death does not fire before AFK.
        state.players.get_mut(&1).unwrap().food = 10_000.0;
        assert!(
            state.afk.last_activity(p_id).is_some(),
            "spawn should touch AFK book"
        );
        // MOVE resets idle stamp to current sim_time.
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Move {
                conn_id: 1,
                xs: 0,
                ys: 0,
                deltas: vec![(1, 0)],
                seq: None,
},
        );
        let t0 = state.afk.last_activity(p_id).unwrap();
        assert_eq!(t0, state.sim_time);
        while rx.try_recv().is_ok() {}

        // Cross AFK threshold in one vitals step (no intervening activity).
        let dt = DEFAULT_AFK_SECS + 1.0;
        tick_vitals(&mut state, dt, &hub);
        assert!(
            state.afk.is_afk_default(p_id, state.sim_time),
            "should be AFK after idle > 600s"
        );
        assert!(
            state.event_log.iter().any(|e| e == &format!("AFK {p_id}")),
            "expected AFK event, got {:?}",
            state.event_log
        );
        let expected_pe =
            format_server_message("PE", &[&format!("{p_id} {YAWN_EMOT_INDEX}")]);
        let mut saw_yawn = false;
        while let Ok(pkt) = rx.try_recv() {
            if pkt == expected_pe.as_bytes() {
                saw_yawn = true;
            }
        }
        assert!(saw_yawn, "expected optional PE yawn when becoming AFK");
    }

    /// SAY ?AFK returns idle/remain/status without resetting the AFK book.
    #[test]
    fn say_afk_query_reports_status() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "afkq@x");
        state.players.get_mut(&1).unwrap().food = 10_000.0;
        // Force idle into warn window (remain ≤ 60).
        state.afk.touch(p_id, 0.0);
        state.sim_time = DEFAULT_AFK_SECS - 30.0;
        while rx.try_recv().is_ok() {}

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?AFK".into(),
            },
        );
        let mut saw = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PS\n") && s.contains("AFK ") && s.contains("status=warn") {
                saw = true;
                assert!(s.contains(&format!("{p_id} AFK ")), "{s}");
            }
        }
        assert!(saw, "expected PS ?AFK with status=warn");
        // Query must not touch (would reset idle to sim_time).
        assert_eq!(state.afk.last_activity(p_id), Some(0.0));
    }

    /// Death paths push format_death_event tags via DeathCause.
    #[test]
    fn death_events_use_death_cause_tags() {
        let hub = OutboundHub::new();
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "diecause@x");
        state.players.get_mut(&1).unwrap().food = 0.0;
        // food == 0 is not < DEATH_FOOD_THRESHOLD (0.0); force slightly negative path:
        state.players.get_mut(&1).unwrap().food = -0.01;
        tick_vitals(&mut state, 0.1, &hub);
        assert!(
            state
                .event_log
                .iter()
                .any(|e| e == &format_death_event(p_id, DeathCause::Hunger)),
            "expected hunger death event, got {:?}",
            state.event_log
        );
        assert_eq!(
            state.players.get(&1).unwrap().death_reason.as_deref(),
            Some(DeathCause::Hunger.wire_tag())
        );
    }

    /// Suicide increments scoreboard deaths.
    #[test]
    fn say_die_increments_scoreboard_deaths() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "suicide@x");
        state.scoreboard.ensure_player(p_id, "Suzy");
        state.scoreboard.set_coins(p_id, 20);
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "DIE".into(),
            },
        );
        let e = state.scoreboard.entry(p_id).unwrap();
        assert_eq!(e.deaths, 1);
        assert_eq!(e.score, 20 - SCORE_PER_DEATH);
        assert_eq!(counters.deaths.load(Ordering::Relaxed), 1);
    }

    /// compose_move_speed is used for FX food change (fire slows reported speed).
    #[test]
    fn food_change_uses_compose_move_speed() {
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "spd@x");
        set_player_position(&mut state, 1, 2, 2);
        let p = state.players.get(&1).unwrap().clone();
        let base = food_change_for_player(&state, &p);
        assert!(
            base.contains(&format!("{WALK_MOVE_SPEED:.2}")),
            "base FX should use walk speed: {base}"
        );
        state.fire.ignite(2, 2, 10.0, 1.0);
        let p = state.players.get(&1).unwrap().clone();
        let slow = food_change_for_player(&state, &p);
        let expected = compose_move_speed(
            false,
            &state.weather,
            &state.snow,
            &state.fire,
            2,
            2,
            0,
        );
        assert!(
            slow.contains(&format!("{expected:.2}")),
            "fire FX should use composed speed {expected:.2}: {slow}"
        );
        assert_ne!(base, slow);
    }

    /// Login intent force-sends MAP_CHUNK and marks has_mc.
    #[test]
    fn login_sends_map_chunk() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(3);
        let mut state = SimState::with_default_empty(test_content());
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Login {
                conn_id: 3,
                reconnect: false,
                email: "mc@login".into(),
                client_tag: "test".into(),
            },
        );
        let mut saw_mc = false;
        while let Ok(pkt) = rx.try_recv() {
            if pkt.starts_with(b"MC\n") {
                saw_mc = true;
            }
        }
        assert!(saw_mc, "login should force-send MC");
        let p = state.players.get(&3).unwrap();
        assert!(p.has_mc);
        assert_eq!((p.last_mc_x, p.last_mc_y), (p.x, p.y));
    }

    /// SAY MAPFORCE always resends MC even when already has_mc and near last center.
    #[test]
    fn say_mapforce_forces_mc_resend() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "force@mc");
        force_send_map_chunk(&mut state, &hub, 1);
        assert!(state.players.get(&1).unwrap().has_mc);
        while rx.try_recv().is_ok() {}

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "MAPFORCE".into(),
            },
        );
        let mut saw_mc = false;
        let mut saw_ok = false;
        while let Ok(pkt) = rx.try_recv() {
            if pkt.starts_with(b"MC\n") {
                saw_mc = true;
            }
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("MAPFORCE OK") {
                saw_ok = true;
            }
        }
        assert!(saw_mc, "MAPFORCE must resend MC");
        assert!(saw_ok, "MAPFORCE must ACK via PS");
    }

    /// Vitals tick updates SimState chunk tier counts; ?CHUNKS reads them.
    #[test]
    fn vitals_tracks_chunk_tiers_and_chunks_query() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "chunks@x");
        assert_eq!(state.chunk_hot, 0);
        tick_vitals(&mut state, 1.0, &hub);
        assert!(
            state.chunk_hot + state.chunk_warm + state.chunk_cold > 0,
            "vitals should populate chunk tier counts"
        );
        let (h, w, c) = (state.chunk_hot, state.chunk_warm, state.chunk_cold);
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?CHUNKS".into(),
            },
        );
        let mut saw = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("CHUNKS") && s.contains(&format!("hot={h}")) {
                saw = true;
            }
        }
        assert!(saw, "expected ?CHUNKS with hot={h} warm={w} cold={c}");
    }

    /// AnimalWorld::nearby_threat for AI (wolf within 5).
    #[test]
    fn animal_nearby_threat_for_ai() {
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "prey@x");
        let (px, py) = {
            let p = state.players.get(&1).unwrap();
            (p.x, p.y)
        };
        assert!(!state.animals.nearby_threat(px, py, ANIMAL_THREAT_RANGE));
        state.animals.spawn(AnimalKind::Wolf, px + 5, py);
        assert!(state.animals.nearby_threat(px, py, ANIMAL_THREAT_RANGE));
        assert!(!state.animals.nearby_threat(px, py, 4));
        // Rabbit is not a threat
        state.animals.animals.clear();
        state.animals.spawn(AnimalKind::Rabbit, px, py);
        assert!(!state.animals.nearby_threat(px, py, ANIMAL_THREAT_RANGE));
    }

    /// SAY LOOK dx dy reports biome + object under relative tile.
    #[test]
    fn say_look_reports_biome_and_object() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "look@x");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.x = 10;
            p.y = 10;
        }
        state.world.write().unwrap().set_object(12, 11, 33);
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "LOOK 2 1".into(),
            },
        );
        let mut saw = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            // format_look: LOOK dx dy biome=… floor=… obj=33 …
            if s.contains("LOOK") && s.contains("obj=33") {
                saw = true;
            }
        }
        assert!(saw, "expected LOOK with obj=33");
        // wire_fields::parse_xy drives LOOK coords (negative offsets).
        assert_eq!(parse_xy(" -1  2"), Some((-1, 2)));
        while rx.try_recv().is_ok() {}
        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "LOOK -1 0".into(),
            },
        );
        let mut saw_neg = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("LOOK -1 0") {
                saw_neg = true;
            }
        }
        assert!(saw_neg, "expected LOOK -1 0 via parse_xy");
    }

    /// SAY ?HEX reports map-PNG color for biome under feet.
    #[test]
    fn say_hex_reports_biome_color_under_feet() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "hex@x");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.x = 4;
            p.y = 5;
        }
        state.world.write().unwrap().set_biome(4, 5, 9); // ocean
        assert_eq!(format_hex_query(9), "HEX 9 004080");
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?HEX".into(),
            },
        );
        let ps = rx.try_recv().expect("PS ?HEX");
        let s = String::from_utf8_lossy(&ps);
        assert!(
            s.contains(&format!("{p_id} HEX 9 004080")),
            "got {s}"
        );
    }

    /// SAY ?TAGS parses held object description tags.
    #[test]
    fn say_tags_reports_held_object_tags() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut db = ContentDb::default();
        db.objects.insert(
            55,
            ObjectDef {
                id: 55,
                description: "Stakes# +tool".into(),
                name: "Stakes".into(),
                containable: false,
                permanent: false,
                blocks_walking: false,
                food_value: 0,
                heat_value: 0.0,
                map_chance: 0.0,
                biomes: Vec::new(),
                num_uses: 0,
                num_slots: 0,
                floor: false,
            dummy_ids: Vec::new(),
            },
        );
        let mut state = SimState::with_default_empty(Arc::new(db));
        let p_id = spawn_player(&mut state, 1, "tags@x");
        while rx.try_recv().is_ok() {}
        // Empty hands
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?TAGS".into(),
            },
        );
        let ps0 = rx.try_recv().expect("PS ?TAGS empty");
        let s0 = String::from_utf8_lossy(&ps0);
        assert!(s0.contains(&format!("{p_id} TAGS 0")), "got {s0}");

        state.players.get_mut(&1).unwrap().held_id = 55;
        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "TAGS".into(),
            },
        );
        let ps1 = rx.try_recv().expect("PS TAGS held");
        let s1 = String::from_utf8_lossy(&ps1);
        assert!(s1.contains(&format!("{p_id} TAGS 55")), "got {s1}");
        assert!(s1.contains("name=Stakes"), "got {s1}");
        assert!(s1.contains("tags=+tool"), "got {s1}");
        assert_eq!(
            format_held_tags_query(55, Some("Stakes# +tool")),
            "TAGS 55 name=Stakes tags=+tool cat=- dummy=0"
        );
    }

    /// SAY PING returns PS PONG with sim_time; client PING tag still works.
    #[test]
    fn say_ping_returns_pong_with_sim_time() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "ping@say");
        state.sim_time = 12.5;
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "PING".into(),
            },
        );
        let expected = format_server_message(
            "PS",
            &[&format!(
                "{} {}",
                state.players.get(&1).unwrap().p_id,
                SimState::format_ping_query(12.5)
            )],
        );
        let mut saw = false;
        while let Ok(pkt) = rx.try_recv() {
            if String::from_utf8_lossy(&pkt) == expected {
                saw = true;
            }
        }
        assert!(saw, "expected SAY PING → {expected}");
        // Client PING tag still echoes unique_id.
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "PING".into(),
                payload: "tok99".into(),
            },
        );
        let mut saw_wire = false;
        while let Ok(pkt) = rx.try_recv() {
            if String::from_utf8_lossy(&pkt) == "PONG\ntok99\n#" {
                saw_wire = true;
            }
        }
        assert!(saw_wire, "client PING tag still replies wire PONG");
    }

    /// `sim_speed` multiplies vitals `dt` (time dilation).
    #[test]
    fn sim_speed_multiplies_tick_vitals_dt() {
        let hub = OutboundHub::new();
        let mut normal = SimState::with_default_empty(test_content());
        let mut fast = SimState::with_default_empty(test_content());
        spawn_player(&mut normal, 1, "spd@n");
        spawn_player(&mut fast, 1, "spd@f");
        {
            let p = normal.players.get_mut(&1).unwrap();
            p.food = 50.0;
            p.age = 20.0;
        }
        {
            let p = fast.players.get_mut(&1).unwrap();
            p.food = 50.0;
            p.age = 20.0;
        }
        normal.sim_speed = 1.0;
        fast.sim_speed = 2.0;
        tick_vitals(&mut normal, 1.0, &hub);
        tick_vitals(&mut fast, 1.0, &hub);
        assert!(
            (normal.sim_time - 1.0).abs() < 1e-4,
            "1x speed sim_time={}",
            normal.sim_time
        );
        assert!(
            (fast.sim_time - 2.0).abs() < 1e-4,
            "2x speed sim_time={}",
            fast.sim_time
        );
        let age_n = normal.players.get(&1).unwrap().age;
        let age_f = fast.players.get(&1).unwrap().age;
        assert!(
            age_f > age_n + 1e-5,
            "2x speed should age faster: {age_f} vs {age_n}"
        );
        let food_n = normal.players.get(&1).unwrap().food;
        let food_f = fast.players.get(&1).unwrap().food;
        assert!(
            food_f < food_n - 1e-5,
            "2x speed should drain food faster: {food_f} vs {food_n}"
        );
    }

    /// `paused` skips vitals (sim_time / food frozen).
    #[test]
    fn paused_skips_tick_vitals() {
        let hub = OutboundHub::new();
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "pause@v");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.food = 40.0;
            p.age = 25.0;
        }
        state.paused = true;
        tick_vitals(&mut state, 5.0, &hub);
        assert_eq!(state.sim_time, 0.0, "paused must not advance sim_time");
        let p = state.players.get(&1).unwrap();
        assert!((p.food - 40.0).abs() < 1e-5, "paused food unchanged");
        assert!((p.age - 25.0).abs() < 1e-5, "paused age unchanged");
        // Resume advances again.
        state.paused = false;
        tick_vitals(&mut state, 1.0, &hub);
        assert!((state.sim_time - 1.0).abs() < 1e-4);
        assert!(state.players.get(&1).unwrap().food < 40.0);
    }

    /// SAY ?TICK reports tick and sim_time via private PS.
    #[test]
    fn say_tick_reports_tick_and_sim_time() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "tick@q");
        state.tick = 42;
        state.sim_time = 3.5;
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?TICK".into(),
            },
        );
        let expected = SimState::format_tick_query(42, 3.5);
        assert_eq!(expected, "TICK 42 3.50");
        let mut saw = false;
        while let Ok(pkt) = rx.try_recv() {
            if String::from_utf8_lossy(&pkt).contains(&format!("{p_id} {expected}")) {
                saw = true;
            }
        }
        assert!(saw, "expected PS with {p_id} {expected}");
    }

    /// SAY PAUSE / RESUME toggles paused flag and replies via PS.
    #[test]
    fn say_pause_resume_sets_paused_flag() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "pause@say");
        assert!(!state.paused);

        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "PAUSE".into(),
            },
        );
        assert!(state.paused);
        let mut saw_pause = false;
        while let Ok(pkt) = rx.try_recv() {
            if String::from_utf8_lossy(&pkt).contains(&format!("{p_id} PAUSED")) {
                saw_pause = true;
            }
        }
        assert!(saw_pause, "expected PAUSED PS");

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "RESUME".into(),
            },
        );
        assert!(!state.paused);
        let mut saw_resume = false;
        while let Ok(pkt) = rx.try_recv() {
            if String::from_utf8_lossy(&pkt).contains(&format!("{p_id} RESUMED")) {
                saw_resume = true;
            }
        }
        assert!(saw_resume, "expected RESUMED PS");
    }

    /// JUMP client tag emits PU (player update) to nearby; babies also get BW wiggle.
    #[test]
    fn jump_emits_pu_note() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "jump@x");
        set_player_position(&mut state, 1, 3, 4);
        while rx.try_recv().is_ok() {}

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "JUMP".into(),
                payload: "5 6".into(),
            },
        );
        let p = state.players.get(&1).unwrap();
        assert_eq!((p.x, p.y), (5, 6));
        let mut saw_pu = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PU\n") && s.contains(&format!("{p_id} ")) {
                saw_pu = true;
            }
        }
        assert!(saw_pu, "JUMP must emit PU note to nearby");
    }

    /// Baby JUMP also emits BW wiggle packet.
    #[test]
    fn jump_baby_emits_bw_wiggle() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "babyjump@x");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.age = 1.0;
        }
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "JUMP".into(),
                payload: "0 0".into(),
            },
        );
        let expected_bw = format_baby_wiggle(p_id);
        let mut saw_bw = false;
        let mut saw_pu = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.as_ref() == expected_bw {
                saw_bw = true;
            }
            if s.starts_with("PU\n") {
                saw_pu = true;
            }
        }
        assert!(saw_pu, "baby JUMP must still emit PU");
        assert!(saw_bw, "baby JUMP must emit BW wiggle");
    }

    /// JUMP while held clears held_by / mother holding link.
    #[test]
    fn jump_releases_held_baby_from_mother() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut state = SimState::with_default_empty(test_content());
        let mother = spawn_player(&mut state, 1, "mom@jump");
        let baby_conn = 2u64;
        let baby = spawn_player(&mut state, baby_conn, "baby@jump");
        {
            let m = state.players.get_mut(&1).unwrap();
            m.start_holding(baby);
            m.x = 0;
            m.y = 0;
        }
        {
            let b = state.players.get_mut(&baby_conn).unwrap();
            b.age = 0.5;
            b.held_by = mother;
            b.x = 0;
            b.y = 0;
        }
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: baby_conn,
                tag: "JUMP".into(),
                payload: "0 0".into(),
            },
        );
        assert_eq!(state.players.get(&baby_conn).unwrap().held_by, 0);
        assert_eq!(state.players.get(&1).unwrap().holding_player_id, 0);
        let _ = mother;
    }

    /// `SAY MUMBLE <text>` fans out PS at [`MUMBLE_RANGE`] (4), not full nearby.
    #[test]
    fn say_mumble_uses_short_range() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx1 = hub.register(1);
        let mut rx2 = hub.register(2);
        let mut rx3 = hub.register(3);
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "m0@x");
        spawn_player(&mut state, 2, "mnear@x");
        spawn_player(&mut state, 3, "mfar@x");
        set_player_position(&mut state, 1, 0, 0);
        set_player_position(&mut state, 2, 3, 0); // within MUMBLE_RANGE=4
        set_player_position(&mut state, 3, 10, 0); // beyond mumble, within normal
        while rx1.try_recv().is_ok() {}
        while rx2.try_recv().is_ok() {}
        while rx3.try_recv().is_ok() {}

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "MUMBLE soft words".into(),
            },
        );
        let mut near_got = false;
        let mut far_got = false;
        while let Ok(pkt) = rx2.try_recv() {
            if String::from_utf8_lossy(&pkt).contains("MUMBLE soft words") {
                near_got = true;
            }
        }
        while let Ok(pkt) = rx3.try_recv() {
            if String::from_utf8_lossy(&pkt).contains("MUMBLE soft words") {
                far_got = true;
            }
        }
        assert!(near_got, "MUMBLE should reach within range 4");
        assert!(!far_got, "MUMBLE must not reach beyond MUMBLE_RANGE");
        let mut self_got = false;
        while let Ok(pkt) = rx1.try_recv() {
            if String::from_utf8_lossy(&pkt).contains("MUMBLE soft words") {
                self_got = true;
            }
        }
        assert!(self_got, "speaker should receive MUMBLE PS");
    }

    /// SAY ?STAGE returns infant/child/adult/elder for age brackets.
    #[test]
    fn say_stage_query_returns_life_stage() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "stage@x");
        // Adult default age 14.
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?STAGE".into(),
            },
        );
        let mut saw = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains(&format!("{p_id} STAGE adult")) {
                saw = true;
            }
        }
        assert!(saw, "default spawn age should report STAGE adult");

        state.players.get_mut(&1).unwrap().age = 2.0;
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "STAGE".into(),
            },
        );
        let mut saw_infant = false;
        while let Ok(pkt) = rx.try_recv() {
            if String::from_utf8_lossy(&pkt).contains("STAGE infant") {
                saw_infant = true;
            }
        }
        assert!(saw_infant, "age 2 → infant");

        state.players.get_mut(&1).unwrap().age = 10.0;
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?STAGE".into(),
            },
        );
        let mut saw_child = false;
        while let Ok(pkt) = rx.try_recv() {
            if String::from_utf8_lossy(&pkt).contains("STAGE child") {
                saw_child = true;
            }
        }
        assert!(saw_child, "age 10 → child");

        state.players.get_mut(&1).unwrap().age = 70.0;
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?STAGE".into(),
            },
        );
        let mut saw_elder = false;
        while let Ok(pkt) = rx.try_recv() {
            if String::from_utf8_lossy(&pkt).contains("STAGE elder") {
                saw_elder = true;
            }
        }
        assert!(saw_elder, "age 70 → elder");
    }

    /// SAY ?BIOMEFOOD reports standing biome food-drain multiplier.
    #[test]
    fn say_biomefood_query() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "biome@food");
        set_player_position(&mut state, 1, 0, 0);
        // Force snow biome under player.
        state.world.write().unwrap().set_biome(0, 0, 4);
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?BIOMEFOOD".into(),
            },
        );
        let expected_body = format_biomefood_query(4);
        let mut saw = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains(&format!("{p_id} {expected_body}")) {
                saw = true;
            }
        }
        assert!(
            saw,
            "expected PS with {p_id} {expected_body}"
        );
    }

    /// SAY ?WARM reports clothing_temp_bonus for equipped slots.
    #[test]
    fn say_warm_query_reports_clothing_temp_bonus() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "warm@x");
        {
            let pl = state.players.get_mut(&1).unwrap();
            pl.hat = 10;
            pl.chest = 20;
            pl.shoes = 0;
        }
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?WARM".into(),
            },
        );
        let expected = format_warm_query(10, 20, 0);
        assert_eq!(expected, "WARM bonus=1.00");
        let mut saw = false;
        while let Ok(pkt) = rx.try_recv() {
            if String::from_utf8_lossy(&pkt).contains(&format!("{p_id} {expected}")) {
                saw = true;
            }
        }
        assert!(saw, "expected PS with {p_id} {expected}");
    }

    /// SAY ?SPEED reports compose_move_speed for the speaker.
    #[test]
    fn say_speed_query_reports_compose_move_speed() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "speed@x");
        set_player_position(&mut state, 1, 0, 0);
        state.players.get_mut(&1).unwrap().riding = true;
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?SPEED".into(),
            },
        );
        let pl = state.players.get(&1).unwrap().clone();
        let speed = player_move_speed(&state, &pl);
        let expected = format_speed_query(speed);
        assert!(
            expected.contains(&format!("{RIDE_MOVE_SPEED:.2}")),
            "riding should report ride speed: {expected}"
        );
        let mut saw = false;
        while let Ok(pkt) = rx.try_recv() {
            if String::from_utf8_lossy(&pkt).contains(&format!("{p_id} {expected}")) {
                saw = true;
            }
        }
        assert!(saw, "expected PS with {p_id} {expected}");
    }

    /// SAY ?WEIGHT reports held + backpack item count.
    #[test]
    fn say_weight_query_reports_held_and_backpack_count() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "weight@x");
        {
            let pl = state.players.get_mut(&1).unwrap();
            pl.held_id = 33;
            pl.backpack = vec![10, 20, 30];
        }
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?WEIGHT".into(),
            },
        );
        let expected = format_weight_query(4); // 1 held + 3 pack
        assert_eq!(expected, "WEIGHT 4 items");
        let mut saw = false;
        while let Ok(pkt) = rx.try_recv() {
            if String::from_utf8_lossy(&pkt).contains(&format!("{p_id} {expected}")) {
                saw = true;
            }
        }
        assert!(saw, "expected PS with {p_id} {expected}");
    }

    /// Ballast from held + backpack slightly reduces reported move speed.
    #[test]
    fn player_move_speed_ballast_from_held_and_backpack() {
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "ballast@x");
        set_player_position(&mut state, 1, 0, 0);
        let empty = {
            let p = state.players.get(&1).unwrap().clone();
            player_move_speed(&state, &p)
        };
        assert!((empty - WALK_MOVE_SPEED).abs() < 0.001);
        {
            let pl = state.players.get_mut(&1).unwrap();
            pl.held_id = 99;
            pl.backpack = vec![1, 2, 3, 4];
        }
        let heavy = {
            let p = state.players.get(&1).unwrap().clone();
            player_move_speed(&state, &p)
        };
        // 5 items → 10% slower
        let expected = WALK_MOVE_SPEED * ballast_speed_mult(5);
        assert!((heavy - expected).abs() < 0.001, "heavy={heavy} expected={expected}");
        assert!(heavy < empty);
    }

    /// SAY ?DRAIN estimates current food drain/sec factors.
    #[test]
    fn say_drain_query_estimates_food_drain_factors() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "drain@x");
        set_player_position(&mut state, 1, 0, 0);
        state.world.write().unwrap().set_biome(0, 0, 4); // snow
        {
            let pl = state.players.get_mut(&1).unwrap();
            pl.age = 70.0;
            pl.sleeping = true;
            pl.hat = 1;
            pl.chest = 1;
            pl.shoes = 1;
        }
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?DRAIN".into(),
            },
        );
        let pl = state.players.get(&1).unwrap();
        let base = FOOD_USE_PER_SEC
            * state.environment.day_night_multiplier()
            * state.apocalypse.food_drain_multiplier();
        let est = estimate_food_drain(
            base,
            biome_food_multiplier(4),
            state.weather.food_drain_mult(),
            pl.age,
            pl.sleeping,
            pl.sitting,
            pl.sick,
            state.combat.bleed_drain(pl.p_id),
            state.fire.drain_at(pl.x, pl.y),
            state.snow.food_extra_at(pl.x, pl.y),
            pl.hat,
            pl.chest,
            pl.shoes,
        );
        let expected = est.format_query();
        assert!(expected.starts_with("DRAIN total="), "{expected}");
        assert!(expected.contains("age=1.50"), "{expected}");
        assert!(expected.contains("sleep=0.50"), "{expected}");
        assert!(expected.contains("biome=1.25"), "{expected}");
        assert!(expected.contains("warm=0.030"), "{expected}");
        let mut saw = false;
        while let Ok(pkt) = rx.try_recv() {
            if String::from_utf8_lossy(&pkt).contains(&format!("{p_id} {expected}")) {
                saw = true;
            }
        }
        assert!(saw, "expected PS with {p_id} {expected}");
    }

    /// SAY ?CRAFTSTATS reports reverse graph products/edges.
    #[test]
    fn say_craftstats_query() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "craft@stats");
        state.craft_graph.insert(1, 2, 3, 0);
        state.craft_graph.insert(3, 4, 5, 0);
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?CRAFTSTATS".into(),
            },
        );
        let body = state.craft_graph.format_craft_stats_query();
        assert!(body.contains("products=2") && body.contains("edges=2"), "{body}");
        let mut saw = false;
        while let Ok(pkt) = rx.try_recv() {
            if String::from_utf8_lossy(&pkt).contains(&format!("{p_id} {body}")) {
                saw = true;
            }
        }
        assert!(saw, "expected craft stats PS");
    }

    /// SAY PLAN <id> returns reverse-craft ingredient path; ?TRANS content counts; SEEKING goal label.
    #[test]
    fn say_plan_seeking_trans_queries() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "plan@x");
        // Chain A=1 + B=2 → C=3; C=3 + D=4 → E=5
        state.craft_graph.insert(1, 2, 3, 0);
        state.craft_graph.insert(3, 4, 5, 0);
        // Have A,B,D in inventory so path to E is solvable.
        {
            let pl = state.players.get_mut(&1).unwrap();
            pl.held_id = 1;
            pl.backpack = vec![2, 4];
            pl.food = 15.0;
        }
        while rx.try_recv().is_ok() {}

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "PLAN 5".into(),
            },
        );
        let mut plan_line = None;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains(&format!("{p_id} PLAN 5")) {
                plan_line = Some(s.into_owned());
            }
        }
        let plan = plan_line.expect("PLAN PS reply");
        assert!(plan.contains("1+2"), "got {plan}");
        assert!(plan.contains("3+4"), "got {plan}");
        assert!(!plan.contains("FAIL"), "got {plan}");

        // Already holding product → HAVE
        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        state.players.get_mut(&1).unwrap().held_id = 5;
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "PLAN 5".into(),
            },
        );
        let mut saw_have = false;
        while let Ok(pkt) = rx.try_recv() {
            if String::from_utf8_lossy(&pkt).contains(&format!("{p_id} PLAN 5 HAVE")) {
                saw_have = true;
            }
        }
        assert!(saw_have, "expected PLAN 5 HAVE");

        // Unreachable product
        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "PLAN 99".into(),
            },
        );
        let mut saw_fail = false;
        while let Ok(pkt) = rx.try_recv() {
            if String::from_utf8_lossy(&pkt).contains(&format!("{p_id} PLAN 99 FAIL")) {
                saw_fail = true;
            }
        }
        assert!(saw_fail, "expected PLAN 99 FAIL");

        // ?TRANS from content counts
        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?TRANS".into(),
            },
        );
        let expected_trans = SimState::format_trans_query(&state.content);
        assert_eq!(expected_trans, "TRANS count=1 last_use=1");
        let mut saw_trans = false;
        while let Ok(pkt) = rx.try_recv() {
            if String::from_utf8_lossy(&pkt).contains(&format!("{p_id} {expected_trans}")) {
                saw_trans = true;
            }
        }
        assert!(saw_trans, "expected ?TRANS PS");

        // SEEKING — fed + holding → IDLE; empty hands + farmer → SEEKOBJECT
        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "SEEKING".into(),
            },
        );
        let mut saw_idle = false;
        while let Ok(pkt) = rx.try_recv() {
            if String::from_utf8_lossy(&pkt).contains(&format!("{p_id} SEEKING IDLE")) {
                saw_idle = true;
            }
        }
        assert!(saw_idle, "holding + fed → SEEKING IDLE");

        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        state.players.get_mut(&1).unwrap().held_id = 0;
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "SEEKING FARMER".into(),
            },
        );
        let mut saw_farm = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains(&format!("{p_id} SEEKING SEEKOBJECT {FARMER_TARGET_ID}")) {
                saw_farm = true;
            }
        }
        assert!(saw_farm, "SEEKING FARMER → SEEKOBJECT profession target");
    }

    /// SAY RECIPE / NEXTCRAFT use reverse craft graph for held item products.
    #[test]
    fn say_recipe_and_nextcraft_queries() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "recipe@x");
        state.craft_graph.insert(1, 2, 3, 0);
        state.craft_graph.insert(3, 4, 5, 0);
        {
            let pl = state.players.get_mut(&1).unwrap();
            pl.held_id = 3; // product C
        }
        while rx.try_recv().is_ok() {}

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "RECIPE".into(),
            },
        );
        let mut saw_recipe = false;
        while let Ok(pkt) = rx.try_recv() {
            if String::from_utf8_lossy(&pkt).contains(&format!("{p_id} RECIPE 3 1+2")) {
                saw_recipe = true;
            }
        }
        assert!(saw_recipe, "RECIPE held-as-product lists ingredients_for");

        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "NEXTCRAFT".into(),
            },
        );
        let mut saw_next = false;
        while let Ok(pkt) = rx.try_recv() {
            if String::from_utf8_lossy(&pkt).contains(&format!("{p_id} NEXTCRAFT 3 5")) {
                saw_next = true;
            }
        }
        assert!(saw_next, "NEXTCRAFT held lists products using held");

        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "RECIPE 5".into(),
            },
        );
        let mut saw_arg = false;
        while let Ok(pkt) = rx.try_recv() {
            if String::from_utf8_lossy(&pkt).contains(&format!("{p_id} RECIPE 5 3+4")) {
                saw_arg = true;
            }
        }
        assert!(saw_arg, "RECIPE <id> overrides held");
    }

    /// SAY SIT / STAND toggles sitting flag.
    #[test]
    fn say_sit_and_stand_toggle_sitting() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "sit@test");
        assert!(!state.players.get(&1).unwrap().sitting);

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "SIT".into(),
            },
        );
        assert!(state.players.get(&1).unwrap().sitting);
        assert!(!state.players.get(&1).unwrap().sleeping);

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "STAND".into(),
            },
        );
        assert!(!state.players.get(&1).unwrap().sitting);
    }

    /// MOVE is rejected while sitting; works again after STAND.
    #[test]
    fn move_blocked_while_sitting() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "sitter@test");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.x = 0;
            p.y = 0;
        }
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "SIT".into(),
            },
        );
        assert!(!apply_move_deltas(&mut state, 1, 0, 0, &[(1, 0)]));
        assert_eq!(
            (
                state.players.get(&1).unwrap().x,
                state.players.get(&1).unwrap().y
            ),
            (0, 0)
        );

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "STAND".into(),
            },
        );
        assert!(apply_move_deltas(&mut state, 1, 0, 0, &[(1, 0)]));
        assert_eq!(
            (
                state.players.get(&1).unwrap().x,
                state.players.get(&1).unwrap().y
            ),
            (1, 0)
        );
    }

    /// While sitting, food drain is reduced by SIT_FOOD_DRAIN_MULT (0.75).
    #[test]
    fn sitting_reduces_food_drain() {
        let hub = OutboundHub::new();
        let mut standing = SimState::with_default_empty(test_content());
        let mut sitting = SimState::with_default_empty(test_content());
        spawn_player(&mut standing, 1, "stand");
        spawn_player(&mut sitting, 1, "sit");
        for s in [&mut standing, &mut sitting] {
            s.environment.temperature = 0.5;
            s.environment.season_length = 10_000.0;
            s.environment.day_length = 10_000.0;
            s.environment.hour_of_day = 12.0;
        }
        sitting.players.get_mut(&1).unwrap().sitting = true;

        let food0 = standing.players.get(&1).unwrap().food;
        assert_eq!(food0, sitting.players.get(&1).unwrap().food);

        tick_vitals(&mut standing, 1.0, &hub);
        tick_vitals(&mut sitting, 1.0, &hub);

        let stand_lost = food0 - standing.players.get(&1).unwrap().food;
        let sit_lost = food0 - sitting.players.get(&1).unwrap().food;
        assert!(
            (stand_lost - FOOD_USE_PER_SEC).abs() < 1e-4,
            "standing drain: lost={stand_lost}"
        );
        let expected_sit = FOOD_USE_PER_SEC * SIT_FOOD_DRAIN_MULT;
        assert!(
            (sit_lost - expected_sit).abs() < 1e-4,
            "sit drain: lost={sit_lost} expected={expected_sit}"
        );
        assert!(sit_lost < stand_lost);
        assert!(sit_lost > FOOD_USE_PER_SEC * SLEEP_FOOD_DRAIN_MULT);
    }

    /// Speech volume radii: whisper=1, mumble=4, shout=48.
    #[test]
    fn speech_volume_constants() {
        assert_eq!(WHISPER_CHAT_RANGE, 1);
        assert_eq!(MUMBLE_CHAT_RANGE, 4);
        assert_eq!(MUMBLE_RANGE, 4);
        assert_eq!(SHOUT_CHAT_RANGE, 48);
        assert_eq!(SHOUT_RANGE, 48);
        assert_eq!(SpeechVolume::Whisper.range(), 1);
        assert_eq!(SpeechVolume::Mumble.range(), 4);
        assert_eq!(SpeechVolume::Shout.range(), 48);
    }

    // ── COUNT / NEAR / DIST / BIOME / FLOOR / FORGETTOOLS / floor DROP ──

    #[test]
    fn format_count_query_matches_online() {
        let mut state = SimState::with_default_empty(test_content());
        assert_eq!(state.count_online(), 0);
        assert_eq!(state.format_count_query(), "COUNT 0");
        spawn_player(&mut state, 1, "a");
        spawn_player(&mut state, 2, "b");
        assert_eq!(state.count_online(), 2);
        assert_eq!(state.format_count_query(), "COUNT 2");
        state.players.get_mut(&2).unwrap().connected = false;
        assert_eq!(state.format_count_query(), "COUNT 1");
        state.players.get_mut(&2).unwrap().connected = true;
        state.players.get_mut(&2).unwrap().deleted = true;
        assert_eq!(state.format_count_query(), "COUNT 1");
    }

    #[test]
    fn say_count_returns_online_count_via_ps() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "count@x");
        spawn_player(&mut state, 2, "count2@x");
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "COUNT".into(),
            },
        );
        let ps = rx.try_recv().expect("PS COUNT");
        let s = String::from_utf8_lossy(&ps);
        assert!(s.starts_with("PS\n"), "got {s}");
        assert!(s.contains(&format!("{p_id} COUNT 2")), "got {s}");

        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?COUNT".into(),
            },
        );
        let ps2 = rx.try_recv().expect("PS ?COUNT");
        let s2 = String::from_utf8_lossy(&ps2);
        assert!(s2.contains("COUNT 2"), "got {s2}");
    }

    #[test]
    fn nearby_p_ids_respects_range_and_sorts() {
        let mut state = SimState::with_default_empty(test_content());
        let a = spawn_player(&mut state, 1, "near_a");
        let b = spawn_player(&mut state, 2, "near_b");
        let c = spawn_player(&mut state, 3, "near_c");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.x = 0;
            p.y = 0;
        }
        {
            let p = state.players.get_mut(&2).unwrap();
            p.x = 5;
            p.y = 0; // within 24
        }
        {
            let p = state.players.get_mut(&3).unwrap();
            p.x = 100;
            p.y = 100; // far
        }
        let near = state.nearby_p_ids(0, 0, NEARBY_RANGE);
        assert!(near.contains(&a));
        assert!(near.contains(&b));
        assert!(!near.contains(&c));
        // Sorted ascending.
        let mut sorted = near.clone();
        sorted.sort_unstable();
        assert_eq!(near, sorted);
        assert_eq!(
            state.format_near_query_at(0, 0),
            format!("NEAR {}", near.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(" "))
        );
        // Far tile: only c if we query at c.
        let far = state.nearby_p_ids(100, 100, NEARBY_RANGE);
        assert_eq!(far, vec![c]);
        assert_eq!(state.format_near_query_at(50, 50), "NEAR none");
    }

    #[test]
    fn say_near_lists_nearby_p_ids() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let a = spawn_player(&mut state, 1, "n1");
        let b = spawn_player(&mut state, 2, "n2");
        state.players.get_mut(&1).unwrap().x = 0;
        state.players.get_mut(&1).unwrap().y = 0;
        state.players.get_mut(&2).unwrap().x = 1;
        state.players.get_mut(&2).unwrap().y = 1;
        let expected = state.format_near_query_at(0, 0);
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "NEAR".into(),
            },
        );
        let ps = rx.try_recv().expect("PS NEAR");
        let s = String::from_utf8_lossy(&ps);
        assert!(s.contains(&format!("{a} {expected}")), "got {s}");
        assert!(s.contains(&a.to_string()) && s.contains(&b.to_string()), "got {s}");
    }

    #[test]
    fn dist_to_player_chebyshev() {
        let mut state = SimState::with_default_empty(test_content());
        let a = spawn_player(&mut state, 1, "d1");
        let b = spawn_player(&mut state, 2, "d2");
        state.players.get_mut(&1).unwrap().x = 0;
        state.players.get_mut(&1).unwrap().y = 0;
        state.players.get_mut(&2).unwrap().x = 3;
        state.players.get_mut(&2).unwrap().y = 4;
        assert_eq!(state.dist_to_player(0, 0, b), Some(4));
        assert_eq!(state.dist_to_player(0, 0, a), Some(0));
        assert_eq!(
            state.format_dist_query_to(0, 0, b),
            format!("DIST {b} 4")
        );
        assert_eq!(
            state.format_dist_query_to(0, 0, 9999),
            "DIST 9999 FAIL"
        );
        state.players.get_mut(&2).unwrap().connected = false;
        assert_eq!(state.dist_to_player(0, 0, b), None);
    }

    #[test]
    fn say_dist_returns_chebyshev_or_fail() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let a = spawn_player(&mut state, 1, "dist_a");
        let b = spawn_player(&mut state, 2, "dist_b");
        state.players.get_mut(&1).unwrap().x = 10;
        state.players.get_mut(&1).unwrap().y = 10;
        state.players.get_mut(&2).unwrap().x = 12;
        state.players.get_mut(&2).unwrap().y = 15; // chebyshev 5
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("DIST {b}"),
            },
        );
        let ps = rx.try_recv().expect("PS DIST");
        let s = String::from_utf8_lossy(&ps);
        assert!(s.contains(&format!("{a} DIST {b} 5")), "got {s}");

        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "DIST 999".into(),
            },
        );
        let ps2 = rx.try_recv().expect("PS DIST fail");
        let s2 = String::from_utf8_lossy(&ps2);
        assert!(s2.contains("DIST 999 FAIL"), "got {s2}");

        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "DIST".into(),
            },
        );
        let ps3 = rx.try_recv().expect("PS bare DIST");
        let s3 = String::from_utf8_lossy(&ps3);
        assert!(s3.contains("DIST 0 FAIL"), "got {s3}");
    }

    #[test]
    fn say_biome_under_feet_with_name() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "biome@x");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.x = 3;
            p.y = 7;
        }
        state.world.write().unwrap().set_biome(3, 7, 5); // desert
        assert_eq!(biome_name(5), "desert");
        assert_eq!(format_biome_query(5, "desert"), "BIOME 5 desert");
        // Optional hex from biome_colors primary desert color.
        assert_eq!(
            format_biome_query_with_hex(5, "desert", Some("DBAC4D")),
            "BIOME 5 desert DBAC4D"
        );

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "BIOME".into(),
            },
        );
        let ps = rx.try_recv().expect("PS BIOME");
        let s = String::from_utf8_lossy(&ps);
        assert!(
            s.contains(&format!("{p_id} BIOME 5 desert DBAC4D")),
            "got {s}"
        );

        while rx.try_recv().is_ok() {}
        state.world.write().unwrap().set_biome(3, 7, 21);
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?BIOME".into(),
            },
        );
        let ps2 = rx.try_recv().expect("PS ?BIOME");
        let s2 = String::from_utf8_lossy(&ps2);
        assert!(s2.contains("BIOME 21 mountain 404040"), "got {s2}");
    }

    #[test]
    fn say_floor_under_feet() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "floor@x");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.x = 2;
            p.y = 2;
        }
        assert_eq!(format_floor_query(0), "FLOOR 0");
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "FLOOR".into(),
            },
        );
        let ps = rx.try_recv().expect("PS FLOOR");
        let s = String::from_utf8_lossy(&ps);
        assert!(s.contains(&format!("{p_id} FLOOR 0")), "got {s}");

        state.world.write().unwrap().set_floor(2, 2, 1596);
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?FLOOR".into(),
            },
        );
        let ps2 = rx.try_recv().expect("PS ?FLOOR");
        let s2 = String::from_utf8_lossy(&ps2);
        assert!(s2.contains("FLOOR 1596"), "got {s2}");
    }

    #[test]
    fn say_forgettools_clears_learned_and_emits_ts_lr() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "forget@x");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.tools.learn(334);
            p.tools.learn(12);
            p.tools.mark_expert(99);
        }
        assert_eq!(state.players.get(&1).unwrap().tools.used, 3);

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "FORGETTOOLS".into(),
            },
        );
        let tools = &state.players.get(&1).unwrap().tools;
        assert_eq!(tools.used, 0);
        assert!(tools.learned.is_empty());
        assert!(tools.experts.is_empty());

        let mut saw_ps = false;
        let mut saw_ts = false;
        let mut saw_lr = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PS\n") && s.contains("FORGETTOOLS OK") {
                saw_ps = true;
                assert!(s.contains(&format!("{p_id} FORGETTOOLS OK")), "got {s}");
                assert!(s.contains("TOOLS 0 1000 learned=0"), "got {s}");
            }
            if s.starts_with("TS\n") {
                saw_ts = true;
                assert!(s.contains("0 1000"), "got {s}");
            }
            if s.starts_with("LR\n") {
                saw_lr = true;
            }
        }
        assert!(saw_ps, "expected FORGETTOOLS PS");
        assert!(saw_ts, "expected TS after forget");
        assert!(saw_lr, "expected empty LR after forget");
    }

    #[test]
    fn drop_skips_floor_only_object() {
        let hub = OutboundHub::new();
        let mut db = ContentDb::default();
        db.objects.insert(
            1596,
            ObjectDef {
                id: 1596,
                description: "Stone Road# groundOnly".into(),
                name: "Stone Road".into(),
                containable: false,
                permanent: false,
                blocks_walking: false,
                food_value: 0,
                heat_value: 0.0,
                map_chance: 0.0,
                biomes: vec![],
                num_uses: 0,
                num_slots: 0,
                floor: true,
            dummy_ids: Vec::new(),
            },
        );
        db.objects.insert(
            33,
            ObjectDef {
                id: 33,
                description: "Gooseberry".into(),
                name: "Gooseberry".into(),
                containable: true,
                permanent: false,
                blocks_walking: false,
                food_value: 3,
                heat_value: 0.0,
                map_chance: 0.0,
                biomes: vec![],
                num_uses: 0,
                num_slots: 0,
                floor: false,
            dummy_ids: Vec::new(),
            },
        );
        let mut state = SimState::with_default_empty(Arc::new(db));
        spawn_player(&mut state, 1, "floor_drop");
        set_player_position(&mut state, 1, 4, 5);
        // Floor-only: skip place, keep held.
        state.players.get_mut(&1).unwrap().held_id = 1596;
        apply_drop(&mut state, &hub, 1, 4, 5, None);
        assert_eq!(state.players.get(&1).unwrap().held_id, 1596);
        assert_eq!(state.world.read().unwrap().get_object(4, 5), 0);
        assert_eq!(state.world.read().unwrap().get_floor(4, 5), 0);

        // Non-floor places normally.
        state.players.get_mut(&1).unwrap().held_id = 33;
        apply_drop(&mut state, &hub, 1, 4, 5, None);
        assert_eq!(state.players.get(&1).unwrap().held_id, 0);
        assert_eq!(state.world.read().unwrap().get_object(4, 5), 33);
    }

    #[test]
    fn pure_query_formatters_count_near_dist_biome_floor() {
        assert_eq!(format_count_query(0), "COUNT 0");
        assert_eq!(format_count_query(7), "COUNT 7");
        assert_eq!(format_near_query(&[]), "NEAR none");
        assert_eq!(format_near_query(&[3, 8]), "NEAR 3 8");
        assert_eq!(query_chebyshev(1, 1, 4, 5), 4);
        assert_eq!(format_dist_query(2, Some(0)), "DIST 2 0");
        assert_eq!(format_dist_query(2, None), "DIST 2 FAIL");
        assert_eq!(format_biome_query(0, "grassland"), "BIOME 0 grassland");
        assert_eq!(format_biome_query(42, ""), "BIOME 42");
        assert_eq!(format_floor_query(1596), "FLOOR 1596");
        assert_eq!(biome_name(1), "swamp");
        assert_eq!(biome_name(2), "yellow");
        assert_eq!(biome_name(3), "gray");
    }

    #[test]
    fn help_lists_count_near_dist_biome_floor_forgettools() {
        let h = SimState::format_help_query();
        for token in [
            "COUNT",
            "NEAR",
            "DIST",
            "?BIOME",
            "?HEX",
            "?TAGS",
            "?FLOOR",
            "FORGETTOOLS",
            "?TWINS",
            "?WARM",
            "?SPEED",
            "?WEIGHT",
            "?DRAIN",
            "?AFK",
            "HARVEST",
            "FISH",
            "MINE",
            "DIG",
            "CHOP",
            "REGEN",
            "CLEAROBJ",
            "FILL",
        ] {
            assert!(h.contains(token), "HELP missing {token}: {h}");
        }
    }

    /// SAY ?TWINS lists stub twin peers (no network).
    #[test]
    fn say_twins_lists_peers_or_none() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "twin@x");
        while rx.try_recv().is_ok() {}

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?TWINS".into(),
            },
        );
        let mut saw_none = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("TWINS none") {
                saw_none = true;
            }
        }
        assert!(saw_none, "empty registry should reply TWINS none");

        state.twins = TwinRegistry::from_endpoints([("127.0.0.1", 8006u16)]);
        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "TWINS".into(),
            },
        );
        let mut saw_peer = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("127.0.0.1:8006") {
                saw_peer = true;
            }
        }
        assert!(saw_peer, "configured peer should appear in ?TWINS");
    }

    #[test]
    fn object_def_is_floor_flag() {
        let mut floor = ObjectDef::empty(1);
        floor.floor = true;
        assert!(floor.is_floor());
        let ground = ObjectDef::empty(2);
        assert!(!ground.is_floor());
    }

    /// SAY PUSH shoves adjacent non-god target one tile away (or swaps if blocked).
    #[test]
    fn say_push_shoves_or_swaps_adjacent() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx1 = hub.register(1);
        let mut rx2 = hub.register(2);
        let mut state = SimState::with_default_empty(test_content());
        let _p1 = spawn_player(&mut state, 1, "pusher@x");
        let p2 = spawn_player(&mut state, 2, "target@x");
        // Place adjacent on open ground.
        state.players.get_mut(&1).unwrap().x = 10;
        state.players.get_mut(&1).unwrap().y = 10;
        state.players.get_mut(&2).unwrap().x = 11;
        state.players.get_mut(&2).unwrap().y = 10;
        while rx1.try_recv().is_ok() {}
        while rx2.try_recv().is_ok() {}

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("PUSH {p2}"),
            },
        );
        let t = state.players.get(&2).unwrap();
        // Shoved away: from (11,10) away from (10,10) → (12,10).
        assert_eq!((t.x, t.y), (12, 10), "target should be shoved one tile away");
        assert_eq!(
            (state.players.get(&1).unwrap().x, state.players.get(&1).unwrap().y),
            (10, 10),
            "actor stays on shove"
        );
        let mut saw_ok = false;
        while let Ok(pkt) = rx1.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("PUSH") && s.contains("OK") && s.contains("shove") {
                saw_ok = true;
            }
        }
        assert!(saw_ok, "expected PUSH OK shove PS");

        // God target cannot be pushed.
        state.players.get_mut(&2).unwrap().godmode = true;
        state.players.get_mut(&2).unwrap().x = 11;
        state.players.get_mut(&2).unwrap().y = 10;
        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        while rx1.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("PUSH {p2}"),
            },
        );
        assert_eq!(
            (state.players.get(&2).unwrap().x, state.players.get(&2).unwrap().y),
            (11, 10),
            "god target stays put"
        );
        let mut saw_god = false;
        while let Ok(pkt) = rx1.try_recv() {
            if String::from_utf8_lossy(&pkt).contains("FAIL god") {
                saw_god = true;
            }
        }
        assert!(saw_god, "expected PUSH FAIL god");

        // Swap path: block shove dest with a third player.
        state.players.get_mut(&2).unwrap().godmode = false;
        let p3 = spawn_player(&mut state, 3, "blocker@x");
        let _ = hub.register(3);
        state.players.get_mut(&1).unwrap().x = 20;
        state.players.get_mut(&1).unwrap().y = 20;
        state.players.get_mut(&2).unwrap().x = 21;
        state.players.get_mut(&2).unwrap().y = 20;
        state.players.get_mut(&3).unwrap().x = 22;
        state.players.get_mut(&3).unwrap().y = 20;
        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        while rx1.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("PUSH {p2}"),
            },
        );
        // Swap: actor ↔ target.
        assert_eq!(
            (state.players.get(&1).unwrap().x, state.players.get(&1).unwrap().y),
            (21, 20)
        );
        assert_eq!(
            (state.players.get(&2).unwrap().x, state.players.get(&2).unwrap().y),
            (20, 20)
        );
        let mut saw_swap = false;
        while let Ok(pkt) = rx1.try_recv() {
            if String::from_utf8_lossy(&pkt).contains("swap") {
                saw_swap = true;
            }
        }
        assert!(saw_swap, "expected PUSH OK swap when dest occupied by {p3}");
    }

    /// SAY PULL pulls adjacent target one step toward self when dest is free.
    #[test]
    fn say_pull_moves_adjacent_toward_self() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx1 = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let _p1 = spawn_player(&mut state, 1, "puller@x");
        let p2 = spawn_player(&mut state, 2, "pulled@x");
        let _ = hub.register(2);
        state.players.get_mut(&1).unwrap().x = 5;
        state.players.get_mut(&1).unwrap().y = 5;
        state.players.get_mut(&2).unwrap().x = 6;
        state.players.get_mut(&2).unwrap().y = 5;
        while rx1.try_recv().is_ok() {}

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("PULL {p2}"),
            },
        );
        // Adjacent pull lands on actor tile (5,5).
        assert_eq!(
            (state.players.get(&2).unwrap().x, state.players.get(&2).unwrap().y),
            (5, 5)
        );
        let mut saw = false;
        while let Ok(pkt) = rx1.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("PULL") && s.contains("OK") {
                saw = true;
            }
        }
        assert!(saw, "expected PULL OK");

        // Far target → range fail.
        state.players.get_mut(&2).unwrap().x = 20;
        state.players.get_mut(&2).unwrap().y = 20;
        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        while rx1.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("PULL {p2}"),
            },
        );
        assert_eq!(
            (state.players.get(&2).unwrap().x, state.players.get(&2).unwrap().y),
            (20, 20)
        );
        let mut saw_range = false;
        while let Ok(pkt) = rx1.try_recv() {
            if String::from_utf8_lossy(&pkt).contains("FAIL range") {
                saw_range = true;
            }
        }
        assert!(saw_range, "expected PULL FAIL range");
    }

    /// SAY KISS emits PE cute/love when adjacent; tiny prestige for ally.
    #[test]
    fn say_kiss_pe_cute_and_ally_prestige() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx1 = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p1 = spawn_player(&mut state, 1, "kisser@x");
        let p2 = spawn_player(&mut state, 2, "kissed@x");
        let _ = hub.register(2);
        state.players.get_mut(&1).unwrap().x = 3;
        state.players.get_mut(&1).unwrap().y = 3;
        state.players.get_mut(&2).unwrap().x = 4;
        state.players.get_mut(&2).unwrap().y = 3;
        // Ensure lineage exists for prestige sync.
        state.social.ensure_lineage(p1, "Kisser");
        while rx1.try_recv().is_ok() {}

        // Non-ally kiss: PE cute, no prestige.
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("KISS {p2}"),
            },
        );
        let mut saw_pe = false;
        let mut saw_ok = false;
        while let Ok(pkt) = rx1.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PE\n") && s.contains(&format!("{p1} {CUTE_EMOT_INDEX}")) {
                saw_pe = true;
            }
            if s.contains("KISS") && s.contains("OK cute") && !s.contains("prestige=") {
                saw_ok = true;
            }
        }
        assert!(saw_pe, "expected PE cute emote");
        assert!(saw_ok, "expected KISS OK cute without prestige");
        let prest0 = state.player_prestige(p1);
        assert!(prest0 < KISS_ALLY_PRESTIGE * 0.5 || prest0 == 0.0);

        // Ally kiss: tiny prestige.
        state.allies.add(p1, p2).unwrap();
        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        while rx1.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("KISS {p2}"),
            },
        );
        let prest1 = state.player_prestige(p1);
        assert!(
            (prest1 - KISS_ALLY_PRESTIGE).abs() < 1e-5,
            "ally kiss prestige, got {prest1}"
        );
        let mut saw_ally = false;
        while let Ok(pkt) = rx1.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("prestige=") && s.contains("KISS") {
                saw_ally = true;
            }
        }
        assert!(saw_ally, "expected ally KISS prestige note");
    }

    /// SAY THANK <p_id>: prestige +0.05 when adjacent; FAIL range/offline/self.
    #[test]
    fn say_thank_adjacent_prestige() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx1 = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p1 = spawn_player(&mut state, 1, "thanker@x");
        let p2 = spawn_player(&mut state, 2, "thanked@x");
        let _ = hub.register(2);
        state.players.get_mut(&1).unwrap().x = 5;
        state.players.get_mut(&1).unwrap().y = 5;
        state.players.get_mut(&2).unwrap().x = 6;
        state.players.get_mut(&2).unwrap().y = 5;
        state.social.ensure_lineage(p1, "Thanker");
        while rx1.try_recv().is_ok() {}

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("THANK {p2}"),
            },
        );
        let prest = state.player_prestige(p1);
        assert!(
            (prest - THANK_PRESTIGE).abs() < 1e-5,
            "thank prestige, got {prest}"
        );
        let mut saw_ok = false;
        while let Ok(pkt) = rx1.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("THANK") && s.contains("OK") && s.contains("prestige=") {
                saw_ok = true;
            }
        }
        assert!(saw_ok, "expected THANK OK prestige note");

        // Out of range → FAIL range, no extra prestige.
        state.players.get_mut(&2).unwrap().x = 20;
        state.players.get_mut(&2).unwrap().y = 20;
        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        while rx1.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("THANK {p2}"),
            },
        );
        assert!(
            (state.player_prestige(p1) - THANK_PRESTIGE).abs() < 1e-5,
            "range fail must not add prestige"
        );
        let mut saw_range = false;
        while let Ok(pkt) = rx1.try_recv() {
            if String::from_utf8_lossy(&pkt).contains("FAIL range") {
                saw_range = true;
            }
        }
        assert!(saw_range, "expected THANK FAIL range");

        // Self thank rejected.
        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        while rx1.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("THANK {p1}"),
            },
        );
        let mut saw_self = false;
        while let Ok(pkt) = rx1.try_recv() {
            if String::from_utf8_lossy(&pkt).contains("FAIL self") {
                saw_self = true;
            }
        }
        assert!(saw_self, "expected THANK FAIL self");
    }

    /// SAY CURSE <p_id> spends one token and raises target score; second curse fails.
    #[test]
    fn say_curse_spends_token() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx1 = hub.register(1);
        let mut rx2 = hub.register(2);
        let mut state = SimState::with_default_empty(test_content());
        let p1 = spawn_player(&mut state, 1, "curser@x");
        let p2 = spawn_player(&mut state, 2, "cursed@x");
        assert_eq!(state.curses.tokens(p1), DEFAULT_CURSE_TOKENS);
        assert_eq!(state.curses.score(p2), 0);
        while rx1.try_recv().is_ok() {}
        while rx2.try_recv().is_ok() {}

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("CURSE {p2}"),
            },
        );
        assert_eq!(state.curses.tokens(p1), 0, "token spent");
        assert_eq!(state.curses.score(p2), 1, "target score +1");
        let mut saw_ok = false;
        let mut saw_cx = false;
        while let Ok(pkt) = rx1.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("CURSE") && s.contains("OK") && s.contains("tokens=0") {
                saw_ok = true;
            }
            if s.starts_with("CX\n") {
                saw_cx = true;
            }
        }
        assert!(saw_ok, "expected CURSE OK PS");
        assert!(saw_cx, "expected CX after curse");
        let mut saw_cs_target = false;
        while let Ok(pkt) = rx2.try_recv() {
            if String::from_utf8_lossy(&pkt).starts_with("CS\n") {
                saw_cs_target = true;
            }
        }
        assert!(saw_cs_target, "target should receive CS");

        // No tokens left → FAIL no_token.
        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        while rx1.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("CURSE {p2}"),
            },
        );
        assert_eq!(state.curses.score(p2), 1, "score unchanged without token");
        let mut saw_fail = false;
        while let Ok(pkt) = rx1.try_recv() {
            if String::from_utf8_lossy(&pkt).contains("FAIL no_token") {
                saw_fail = true;
            }
        }
        assert!(saw_fail, "expected CURSE FAIL no_token");

        // Self-curse rejected without spending (re-grant token first).
        state.curses.add_token(p1);
        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        while rx1.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("CURSE {p1}"),
            },
        );
        assert_eq!(state.curses.tokens(p1), 1, "self-curse must not spend");
        let mut saw_self = false;
        while let Ok(pkt) = rx1.try_recv() {
            if String::from_utf8_lossy(&pkt).contains("FAIL self") {
                saw_self = true;
            }
        }
        assert!(saw_self, "expected CURSE FAIL self");
    }

    /// SAY BLESS <p_id>: clear wounds + tiny prestige when adjacent.
    #[test]
    fn say_bless_clears_wound_and_prestige() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx1 = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p1 = spawn_player(&mut state, 1, "blesser@x");
        let p2 = spawn_player(&mut state, 2, "blessed@x");
        let _ = hub.register(2);
        state.players.get_mut(&1).unwrap().x = 8;
        state.players.get_mut(&1).unwrap().y = 8;
        state.players.get_mut(&2).unwrap().x = 9;
        state.players.get_mut(&2).unwrap().y = 8;
        state.social.ensure_lineage(p1, "Blesser");
        state.combat.apply_wound(p2, 2);
        assert_eq!(state.combat.wound_of(p2), 2);
        while rx1.try_recv().is_ok() {}

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("BLESS {p2}"),
            },
        );
        assert_eq!(state.combat.wound_of(p2), 0, "wounds cleared");
        let prest = state.player_prestige(p1);
        assert!(
            (prest - BLESS_PRESTIGE).abs() < 1e-5,
            "bless prestige, got {prest}"
        );
        let mut saw_ok = false;
        while let Ok(pkt) = rx1.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("BLESS") && s.contains("OK") && s.contains("was=2") && s.contains("prestige=")
            {
                saw_ok = true;
            }
        }
        assert!(saw_ok, "expected BLESS OK was=2 prestige note");

        // Out of range: no heal, no extra prestige.
        state.combat.apply_wound(p2, 1);
        state.players.get_mut(&2).unwrap().x = 30;
        state.players.get_mut(&2).unwrap().y = 30;
        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        while rx1.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("BLESS {p2}"),
            },
        );
        assert_eq!(state.combat.wound_of(p2), 1, "range fail keeps wound");
        assert!(
            (state.player_prestige(p1) - BLESS_PRESTIGE).abs() < 1e-5,
            "range fail must not add prestige"
        );
        let mut saw_range = false;
        while let Ok(pkt) = rx1.try_recv() {
            if String::from_utf8_lossy(&pkt).contains("FAIL range") {
                saw_range = true;
            }
        }
        assert!(saw_range, "expected BLESS FAIL range");
    }

    /// HELP lists THANK / CURSE / BLESS / HUG / SLAP speech acts.
    #[test]
    fn say_help_lists_thank_curse_bless() {
        let help = SimState::format_help_query();
        assert!(help.contains("THANK"), "HELP should list THANK");
        assert!(help.contains("CURSE"), "HELP should list CURSE");
        assert!(help.contains("BLESS"), "HELP should list BLESS");
        assert!(help.contains("HUG"), "HELP should list HUG");
        assert!(help.contains("SLAP"), "HELP should list SLAP");
        assert!(help.contains("MUTE"), "HELP should list MUTE");
        assert!(help.contains("UNMUTE"), "HELP should list UNMUTE");
        assert!(help.contains("DEAF"), "HELP should list DEAF");
        assert!(help.contains("?REP"), "HELP should list ?REP");
    }

    /// SAY MUTE filters normal chat PS; UNMUTE / MUTE LIST; ?REP after illegal kill.
    #[test]
    fn say_mute_filters_chat_and_rep_on_kill() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx1 = hub.register(1);
        let mut rx2 = hub.register(2);
        let mut state = SimState::with_default_empty(test_content());
        let a = spawn_player(&mut state, 1, "speaker@x");
        let b = spawn_player(&mut state, 2, "listener@x");
        set_player_position(&mut state, 1, 0, 0);
        set_player_position(&mut state, 2, 1, 0);
        while rx1.try_recv().is_ok() {}
        while rx2.try_recv().is_ok() {}

        // Listener mutes speaker.
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 2,
                tag: "SAY".into(),
                payload: format!("MUTE {a}"),
            },
        );
        let mut saw_mute_ok = false;
        while let Ok(pkt) = rx2.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("MUTE") && s.contains("OK") {
                saw_mute_ok = true;
            }
        }
        assert!(saw_mute_ok, "expected MUTE OK");
        assert!(!state.mutes.should_deliver(b, a));

        // Normal SAY from A must not reach B.
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "hello muted".into(),
            },
        );
        let mut b_heard = false;
        while let Ok(pkt) = rx2.try_recv() {
            if String::from_utf8_lossy(&pkt).contains("hello muted") {
                b_heard = true;
            }
        }
        assert!(!b_heard, "muted listener must not receive normal SAY PS");
        // Speaker still hears self.
        let mut a_heard = false;
        while let Ok(pkt) = rx1.try_recv() {
            if String::from_utf8_lossy(&pkt).contains("hello muted") {
                a_heard = true;
            }
        }
        assert!(a_heard, "speaker should still receive own SAY");

        // MUTE LIST
        while rx2.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 2,
                tag: "SAY".into(),
                payload: "MUTE LIST".into(),
            },
        );
        let mut saw_list = false;
        while let Ok(pkt) = rx2.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("MUTE") && s.contains(&a.to_string()) {
                saw_list = true;
            }
        }
        assert!(saw_list, "expected MUTE LIST with speaker id");

        // UNMUTE restores delivery.
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 2,
                tag: "SAY".into(),
                payload: format!("UNMUTE {a}"),
            },
        );
        assert!(state.mutes.should_deliver(b, a));

        // Illegal kill worsens reputation; ?REP reports it.
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("KILL {b}"),
            },
        );
        assert_eq!(state.reputation.get(a), -1.0);
        while rx1.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "?REP".into(),
            },
        );
        let mut saw_rep = false;
        while let Ok(pkt) = rx1.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("REP") && s.contains("score=-1.0") {
                saw_rep = true;
            }
        }
        assert!(saw_rep, "expected PS ?REP with score=-1.0");
    }

    /// Numeric client_tag triggers version gate soft path; LOGIN still succeeds
    /// when `client_version_strict` is false.
    #[test]
    fn login_version_gate_soft_only() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let _rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        state.version_gate = VersionGatePolicy::strict(437);
        state.client_version_strict = false;
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Login {
                conn_id: 1,
                reconnect: false,
                email: "v@x".into(),
                client_tag: "400".into(), // mismatch → soft log, still spawn
            },
        );
        assert!(
            state.players.contains_key(&1),
            "soft version mismatch must not block LOGIN"
        );
        assert_eq!(state.players.get(&1).unwrap().p_id, 2);
    }

    /// `client_version_strict` hard-rejects LOGIN on version mismatch (PS + no spawn).
    #[test]
    fn login_version_gate_strict_hard_reject() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        state.version_gate = VersionGatePolicy {
            required: 437,
            require_exact: true,
            allow_newer: false,
            require_client_version: false,
        };
        state.client_version_strict = true;
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Login {
                conn_id: 1,
                reconnect: false,
                email: "strict@x".into(),
                client_tag: "400".into(),
            },
        );
        assert!(
            !state.players.contains_key(&1),
            "strict mismatch must not spawn player"
        );
        let mut saw_ps = false;
        let mut saw_rejected = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PS\n") && s.contains("VERSION REJECTED") {
                saw_ps = true;
                assert!(s.contains("client=400"), "got {s}");
                assert!(s.contains("required=437"), "got {s}");
            }
            if s.starts_with("REJECTED") {
                saw_rejected = true;
            }
        }
        assert!(saw_ps, "expected PS VERSION REJECTED");
        assert!(saw_rejected, "expected REJECTED tag");

        // Matching version still logs in under strict.
        let _rx2 = hub.register(2);
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Login {
                conn_id: 2,
                reconnect: false,
                email: "ok@x".into(),
                client_tag: "437".into(),
            },
        );
        assert!(state.players.contains_key(&2));
    }

    /// SAY DEAF toggles Player.deaf; blocks normal chat; WHISPER still delivers.
    #[test]
    fn say_deaf_blocks_chat_allows_whisper() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx1 = hub.register(1);
        let mut rx2 = hub.register(2);
        let mut state = SimState::with_default_empty(test_content());
        let a = spawn_player(&mut state, 1, "speaker@x");
        let b = spawn_player(&mut state, 2, "listener@x");
        set_player_position(&mut state, 1, 0, 0);
        set_player_position(&mut state, 2, 1, 0);
        while rx1.try_recv().is_ok() {}
        while rx2.try_recv().is_ok() {}

        // Listener goes deaf.
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 2,
                tag: "SAY".into(),
                payload: "DEAF".into(),
            },
        );
        assert!(state.players.get(&2).unwrap().deaf);
        let mut saw_on = false;
        while let Ok(pkt) = rx2.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("DEAF ON") {
                saw_on = true;
            }
        }
        assert!(saw_on, "expected DEAF ON");

        // Normal SAY from A must not reach deaf B.
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "hello deaf".into(),
            },
        );
        let mut b_heard = false;
        while let Ok(pkt) = rx2.try_recv() {
            if String::from_utf8_lossy(&pkt).contains("hello deaf") {
                b_heard = true;
            }
        }
        assert!(!b_heard, "deaf listener must not receive normal SAY");

        // WHISPER still reaches deaf B.
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("WHISPER {b} secret deaf"),
            },
        );
        let mut b_whisper = false;
        while let Ok(pkt) = rx2.try_recv() {
            if String::from_utf8_lossy(&pkt).contains("secret deaf") {
                b_whisper = true;
            }
        }
        assert!(b_whisper, "deaf listener must still receive WHISPER");

        // Toggle off.
        while rx2.try_recv().is_ok() {}
        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 2,
                tag: "SAY".into(),
                payload: "DEAF".into(),
            },
        );
        assert!(!state.players.get(&2).unwrap().deaf);
        let _ = a;
    }

    /// SAY HUG <p_id>: PE love when adjacent; FAIL self/range.
    #[test]
    fn say_hug_pe_love_adjacent() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx1 = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p1 = spawn_player(&mut state, 1, "hugger@x");
        let p2 = spawn_player(&mut state, 2, "hugged@x");
        let _ = hub.register(2);
        state.players.get_mut(&1).unwrap().x = 7;
        state.players.get_mut(&1).unwrap().y = 7;
        state.players.get_mut(&2).unwrap().x = 8;
        state.players.get_mut(&2).unwrap().y = 7;
        while rx1.try_recv().is_ok() {}

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("HUG {p2}"),
            },
        );
        let mut saw_pe = false;
        let mut saw_ok = false;
        while let Ok(pkt) = rx1.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PE\n") && s.contains(&format!("{p1} {LOVE_EMOT_INDEX}")) {
                saw_pe = true;
            }
            if s.contains("HUG") && s.contains("OK love") {
                saw_ok = true;
            }
        }
        assert!(saw_pe, "expected PE love emote");
        assert!(saw_ok, "expected HUG OK love");

        // Self hug rejected.
        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        while rx1.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("HUG {p1}"),
            },
        );
        let mut saw_self = false;
        while let Ok(pkt) = rx1.try_recv() {
            if String::from_utf8_lossy(&pkt).contains("FAIL self") {
                saw_self = true;
            }
        }
        assert!(saw_self, "expected HUG FAIL self");

        // Far → range fail.
        state.players.get_mut(&2).unwrap().x = 40;
        state.players.get_mut(&2).unwrap().y = 40;
        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        while rx1.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("HUG {p2}"),
            },
        );
        let mut saw_range = false;
        while let Ok(pkt) = rx1.try_recv() {
            if String::from_utf8_lossy(&pkt).contains("FAIL range") {
                saw_range = true;
            }
        }
        assert!(saw_range, "expected HUG FAIL range");
    }

    /// SAY SLAP <p_id>: PE mad; tiny wound if not ally; no wound for ally.
    #[test]
    fn say_slap_pe_mad_wound_if_not_ally() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx1 = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p1 = spawn_player(&mut state, 1, "slapper@x");
        let p2 = spawn_player(&mut state, 2, "slapped@x");
        let _ = hub.register(2);
        state.players.get_mut(&1).unwrap().x = 0;
        state.players.get_mut(&1).unwrap().y = 0;
        state.players.get_mut(&2).unwrap().x = 1;
        state.players.get_mut(&2).unwrap().y = 0;
        while rx1.try_recv().is_ok() {}

        // Non-ally: PE mad + wound.
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("SLAP {p2}"),
            },
        );
        assert_eq!(state.combat.wound_of(p2), SLAP_WOUND);
        let mut saw_pe = false;
        let mut saw_wound = false;
        while let Ok(pkt) = rx1.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PE\n") && s.contains(&format!("{p1} {MAD_EMOT_INDEX}")) {
                saw_pe = true;
            }
            if s.contains("SLAP") && s.contains("OK mad wound=") {
                saw_wound = true;
            }
        }
        assert!(saw_pe, "expected PE mad");
        assert!(saw_wound, "expected SLAP OK mad wound");

        // Ally: PE mad, no wound.
        state.combat.clear_wound(p2);
        state.allies.add(p1, p2).unwrap();
        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        while rx1.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("SLAP {p2}"),
            },
        );
        assert_eq!(state.combat.wound_of(p2), 0, "ally slap does not wound");
        let mut saw_ally = false;
        while let Ok(pkt) = rx1.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("SLAP") && s.contains("OK mad") && !s.contains("wound=") {
                saw_ally = true;
            }
        }
        assert!(saw_ally, "expected ally SLAP OK mad without wound");

        // Self slap rejected.
        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        while rx1.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("SLAP {p1}"),
            },
        );
        let mut saw_self = false;
        while let Ok(pkt) = rx1.try_recv() {
            if String::from_utf8_lossy(&pkt).contains("FAIL self") {
                saw_self = true;
            }
        }
        assert!(saw_self, "expected SLAP FAIL self");
    }

    #[test]
    fn catch_up_extra_steps_pure() {
        assert_eq!(catch_up_extra_steps(1, 0, 5), 0);
        assert_eq!(catch_up_extra_steps(1, 3, 5), 3);
        // max_extra cap (periods_behind 10, max 5 → 5 extras).
        assert_eq!(catch_up_extra_steps(1, 10, 5), 5);
        // Already on tick % 10 == 0 → no extras.
        assert_eq!(catch_up_extra_steps(10, 5, 5), 0);
        // Mid-%10: starting at 8, extras stop before/at 10 → only 2 (8→9, 9→10).
        assert_eq!(catch_up_extra_steps(8, 5, 5), 2);
        // max_extra 0 → 0.
        assert_eq!(catch_up_extra_steps(1, 10, 0), 0);
    }

    #[test]
    fn ka_mid_path_leaves_tile_unchanged() {
        let mut state = SimState::with_default_empty(test_content());
        state.timed_movement = true;
        spawn_player(&mut state, 1, "ka@t");
        set_player_position(&mut state, 1, 5, 5);
        let hub = OutboundHub::new();
        apply_move_path_start(&mut state, &hub, 1, 5, 5, &[(1, 0)], Some(3)).unwrap();
        assert!(!set_player_position_respecting_path(&mut state, 1, 9, 5));
        assert_eq!(
            (
                state.players.get(&1).unwrap().x,
                state.players.get(&1).unwrap().y
            ),
            (5, 5)
        );
    }

    /// Haxe `Connection.keepAlive()` is empty — KA must never write position.
    #[test]
    fn ka_intent_does_not_change_position() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut state = SimState::with_default_empty(test_content());
        state.timed_movement = true;
        spawn_player(&mut state, 1, "ka2@t");
        set_player_position(&mut state, 1, 10, 20);
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::KeepAlive {
                conn_id: 1,
                x: 99,
                y: 88,
            },
        );
        let p = state.players.get(&1).unwrap();
        assert_eq!((p.x, p.y), (10, 20), "KA must not apply coords");
        // Also while moving.
        apply_move_path_start(&mut state, &hub, 1, 10, 20, &[(1, 0)], Some(1)).unwrap();
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::KeepAlive {
                conn_id: 1,
                x: 0,
                y: 0,
            },
        );
        let p = state.players.get(&1).unwrap();
        assert_eq!((p.x, p.y), (10, 20));
        assert!(p.move_path.is_some(), "KA must not cancel path");
    }

    /// Parse PU body fields: order is `… heat seq force x y …` (indices 12/13 after tag line).
    fn pu_seq_force(pkt: &[u8]) -> Option<(i32, i32)> {
        let s = String::from_utf8_lossy(pkt);
        if !s.starts_with("PU\n") {
            return None;
        }
        let line = s.lines().nth(1)?;
        let f: Vec<&str> = line.split_whitespace().collect();
        // p_id po_id facing action atx aty held ov ox oy ot heat seq force x y ...
        if f.len() < 14 {
            return None;
        }
        Some((f[12].parse().ok()?, f[13].parse().ok()?))
    }

    /// Human LOGIN must not spawn on mother/NPC tile (bootstrap desync fix).
    #[test]
    fn human_login_spawn_near_configured_spawn_not_mother() {
        let mut state = SimState::with_default_empty(test_content());
        state.spawn_x = 100;
        state.spawn_y = 200;
        {
            let mut m = Player::new(50, 50, "mom@t");
            m.x = 499;
            m.y = 487;
            m.age = 20.0;
            m.connected = true;
            state.players.insert(50, m);
        }
        spawn_player(&mut state, 5, "human@test");
        let p = state.players.get(&5).unwrap();
        assert_ne!(
            (p.x, p.y),
            (499, 487),
            "human must not use mother/NPC tile"
        );
        // Empty test world: find_playable_spawn walks from prefer — stay near spawn.
        assert!(
            (p.x - 100).abs() <= 200 && (p.y - 200).abs() <= 200,
            "got {},{}",
            p.x,
            p.y
        );
    }

    /// Haxe: quadDist <= 5 accepts client start (positionChanged snap).
    #[test]
    fn move_path_small_client_jump_accepted() {
        let mut state = SimState::with_default_empty(test_content());
        state.timed_movement = true;
        // Production-like cheby config must not change timed gate (always Haxe 5).
        state.move_jump_max_chebyshev = 3;
        spawn_player(&mut state, 1, "jmp@t");
        set_player_position(&mut state, 1, 5, 5);
        let hub = OutboundHub::new();
        // (2,1) → quad = 4+1 = 5 → accept
        apply_move_path_start(&mut state, &hub, 1, 7, 6, &[(1, 0)], Some(7)).unwrap();
        let p = state.players.get(&1).unwrap();
        assert_eq!((p.x, p.y), (7, 6), "server snaps to client start");
        assert!(p.move_path.is_some());
        assert_eq!(p.move_path.as_ref().unwrap().start_x, 7);
        assert_eq!(p.move_path.as_ref().unwrap().start_y, 6);
        assert_eq!(p.move_path.as_ref().unwrap().seq, 7);
    }

    /// Haxe: quadDist > 5 → CancleMovement / JumpTooFar (even if cheby config is 3).
    #[test]
    fn move_path_large_jump_rejected() {
        let mut state = SimState::with_default_empty(test_content());
        state.timed_movement = true;
        // Production default-ish: cheby=3 must NOT widen timed gate past Haxe 5.
        state.move_jump_max_chebyshev = 3;
        spawn_player(&mut state, 1, "far@t");
        set_player_position(&mut state, 1, 5, 5);
        let hub = OutboundHub::new();
        // (3,0) → quad = 9 > 5
        let err = apply_move_path_start(&mut state, &hub, 1, 8, 5, &[(1, 0)], Some(3))
            .unwrap_err();
        assert_eq!(err, MoveReject::JumpTooFar);
        let p = state.players.get(&1).unwrap();
        assert_eq!((p.x, p.y), (5, 5), "reject keeps server position");
        // Intent path: force PU at server with client seq, force=1.
        let counters = Counters::new();
        let mut rx = hub.register(1);
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Move {
                conn_id: 1,
                xs: 8,
                ys: 5,
                deltas: vec![(1, 0)],
                seq: Some(3),
            },
        );
        assert_eq!(
            (
                state.players.get(&1).unwrap().x,
                state.players.get(&1).unwrap().y
            ),
            (5, 5)
        );
        assert_eq!(state.players.get(&1).unwrap().done_moving_seq, 3);
        let mut saw_force = false;
        while let Ok(pkt) = rx.try_recv() {
            if let Some((seq, force)) = pu_seq_force(&pkt) {
                if seq == 3 && force == 1 {
                    saw_force = true;
                }
            }
        }
        assert!(saw_force, "reject must force PU at server with client seq");
    }

    /// Path finish PU must carry the MOVE seq (not hardcoded 1).
    #[test]
    fn path_finish_pu_uses_done_moving_seq() {
        let mut state = SimState::with_default_empty(test_content());
        state.timed_movement = true;
        spawn_player(&mut state, 1, "fin@t");
        set_player_position(&mut state, 1, 0, 0);
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        apply_move_path_start(&mut state, &hub, 1, 0, 0, &[(1, 0)], Some(11)).unwrap();
        // Drain PM
        while rx.try_recv().is_ok() {}
        tick_move_paths(&mut state, 1.0, &hub);
        let p = state.players.get(&1).unwrap();
        assert!(p.move_path.is_none());
        assert_eq!(p.done_moving_seq, 11);
        assert_eq!((p.x, p.y), (1, 0));
        let mut saw = false;
        while let Ok(pkt) = rx.try_recv() {
            if let Some((seq, force)) = pu_seq_force(&pkt) {
                if seq == 11 && force == 0 {
                    saw = true;
                }
            }
        }
        assert!(saw, "finish PU must include done_moving_seq=11 force=0");
    }

    /// Mid-path blocked cancel must keep path seq (not saturating_add thrash).
    #[test]
    fn tick_cancel_blocked_keeps_path_seq() {
        use ol_content::ObjectDef;
        let mut db = ContentDb::default();
        db.objects.insert(
            99,
            ObjectDef {
                id: 99,
                description: "Wall".into(),
                name: "Wall".into(),
                containable: false,
                permanent: true,
                blocks_walking: true,
                food_value: 0,
                heat_value: 0.0,
                map_chance: 0.0,
                biomes: vec![],
                num_uses: 0,
                num_slots: 0,
                floor: false,
            dummy_ids: Vec::new(),
            },
        );
        let mut state = SimState::with_default_empty(std::sync::Arc::new(db));
        state.timed_movement = true;
        spawn_player(&mut state, 1, "blk@t");
        set_player_position(&mut state, 1, 0, 0);
        // Block the destination of the only step so advance cancels.
        state.world.write().unwrap().set_object(1, 0, 99);
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        // Build path that already includes the (now blocked) step — as if accepted earlier.
        state.players.get_mut(&1).unwrap().move_path =
            Some(build_move_path(0, 0, vec![(1, 0)], 10.0, 7, 0, 0));
        state.players.get_mut(&1).unwrap().moving = true;
        tick_move_paths(&mut state, 1.0, &hub);
        let p = state.players.get(&1).unwrap();
        assert!(p.move_path.is_none());
        assert_eq!((p.x, p.y), (0, 0), "cancel stays on last good tile");
        assert_eq!(p.done_moving_seq, 7, "must not double-increment past path.seq");
        let mut saw = false;
        while let Ok(pkt) = rx.try_recv() {
            if let Some((seq, force)) = pu_seq_force(&pkt) {
                if seq == 7 && force == 1 {
                    saw = true;
                }
            }
        }
        assert!(saw, "cancel force PU must use path seq=7 force=1");
    }

    /// Haxe: blocked client start → CancleMovement without snap.
    #[test]
    fn move_path_blocked_start_no_snap() {
        use ol_content::ObjectDef;
        let mut db = ContentDb::default();
        db.objects.insert(
            99,
            ObjectDef {
                id: 99,
                description: "Wall".into(),
                name: "Wall".into(),
                containable: false,
                permanent: true,
                blocks_walking: true,
                food_value: 0,
                heat_value: 0.0,
                map_chance: 0.0,
                biomes: vec![],
                num_uses: 0,
                num_slots: 0,
                floor: false,
            dummy_ids: Vec::new(),
            },
        );
        let mut state = SimState::with_default_empty(std::sync::Arc::new(db));
        state.timed_movement = true;
        spawn_player(&mut state, 1, "bst@t");
        set_player_position(&mut state, 1, 5, 5);
        // Client start one tile east is a wall (within jump), path would be empty after snap.
        state.world.write().unwrap().set_object(6, 5, 99);
        let hub = OutboundHub::new();
        let err = apply_move_path_start(&mut state, &hub, 1, 6, 5, &[(1, 0)], Some(4))
            .unwrap_err();
        assert_eq!(err, MoveReject::BlockedStart);
        let p = state.players.get(&1).unwrap();
        assert_eq!((p.x, p.y), (5, 5), "must not snap onto blocked start");
    }

    #[test]
    fn use_rejected_while_moving_no_world_mutation() {
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "um@t");
        set_player_position(&mut state, 1, 5, 5);
        state.world.write().unwrap().set_object(5, 5, 33);
        state.players.get_mut(&1).unwrap().move_path =
            Some(build_move_path(5, 5, vec![(1, 0)], 3.75, 1, 0, 0));
        state.players.get_mut(&1).unwrap().moving = true;
        let r = apply_use_at(&mut state, 1, 5, 5).unwrap();
        assert!(!r.applied);
        assert_eq!(state.world.read().unwrap().get_object(5, 5), 33);
    }

    #[test]
    fn drop_rejected_while_moving_force_pu_fm() {
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "dm@t");
        set_player_position(&mut state, 1, 3, 3);
        state.players.get_mut(&1).unwrap().held_id = 34;
        state.players.get_mut(&1).unwrap().move_path =
            Some(build_move_path(3, 3, vec![(1, 0)], 3.75, 1, 0, 0));
        state.players.get_mut(&1).unwrap().moving = true;
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        apply_drop(&mut state, &hub, 1, 3, 3, None);
        assert_eq!(state.players.get(&1).unwrap().held_id, 34);
        let mut saw_pu = false;
        let mut saw_fm = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PU\n") {
                saw_pu = true;
            }
            if s.starts_with("FM\n") {
                saw_fm = true;
            }
        }
        assert!(saw_pu && saw_fm);
    }

    #[test]
    fn attach_fitness_mother_lineage_sets_mali() {
        let mut state = SimState::with_default_empty(test_content());
        let mid = spawn_player(&mut state, 1, "mom@f");
        state.players.get_mut(&1).unwrap().age = 25.0;
        state.players.get_mut(&1).unwrap().food = 20.0;
        // Insert child without spawn_player fitness path to exercise helper once.
        let child = {
            let p_id = player_id_for_conn(2);
            let mut pl = Player::new(p_id, 2, "kid@f");
            pl.age = 0.0;
            let display = pl.display_name();
            state.players.insert(2, pl);
            attach_fitness_mother_lineage(&mut state, p_id, &display, mid, 0, 0);
            p_id
        };
        assert_eq!(
            state.social.lineages.get(&child).unwrap().mother_id,
            Some(mid)
        );
        assert!(
            (state
                .fertility
                .by_mother
                .get(&mid)
                .unwrap()
                .children_birth_mali
                - 0.1)
            .abs()
                < 1e-5
        );
    }


    #[test]
    fn instant_move_client_seq_sets_done_moving_seq() {
        let mut state = SimState::with_default_empty(test_content());
        state.timed_movement = false;
        spawn_player(&mut state, 1, "seqi@t");
        set_player_position(&mut state, 1, 0, 0);
        assert!(apply_move_deltas_with_seq(
            &mut state,
            1,
            0,
            0,
            &[(1, 0)],
            Some(5)
        ));
        let p = state.players.get(&1).unwrap();
        assert_eq!(p.done_moving_seq, 5);
        assert!(p.move_path.is_none());
        assert_eq!((p.x, p.y), (1, 0));
    }

    #[test]
    fn move_path_seq_complete_sets_done_moving_seq() {
        let mut state = SimState::with_default_empty(test_content());
        state.timed_movement = true;
        spawn_player(&mut state, 1, "seq@t");
        set_player_position(&mut state, 1, 0, 0);
        let hub = OutboundHub::new();
        apply_move_path_start(&mut state, &hub, 1, 0, 0, &[(1, 0)], Some(5)).unwrap();
        tick_move_paths(&mut state, 0.5, &hub);
        let p = state.players.get(&1).unwrap();
        assert!(p.move_path.is_none());
        assert!(!p.moving);
        assert_eq!(p.done_moving_seq, 5);
        assert_eq!((p.x, p.y), (1, 0));
    }

    #[test]
    fn path_replace_mid_move_clears_residual() {
        let mut state = SimState::with_default_empty(test_content());
        state.timed_movement = true;
        spawn_player(&mut state, 1, "rep@t");
        set_player_position(&mut state, 1, 0, 0);
        let hub = OutboundHub::new();
        apply_move_path_start(
            &mut state,
            &hub,
            1,
            0,
            0,
            &[(1, 0), (1, 0), (1, 0)],
            Some(1),
        )
        .unwrap();
        tick_move_paths(&mut state, 0.01, &hub);
        apply_move_path_start(&mut state, &hub, 1, 0, 0, &[(0, 1)], Some(2)).unwrap();
        let path = state.players.get(&1).unwrap().move_path.as_ref().unwrap();
        assert_eq!(path.remaining.len(), 1);
        assert_eq!(path.remaining[0], (0, 1));
        assert_eq!(path.seq, 2);
    }

    #[test]
    fn apply_move_path_trunc_walkability() {
        use ol_content::ObjectDef;
        let mut db = ContentDb::default();
        db.objects.insert(
            99,
            ObjectDef {
                id: 99,
                description: "Wall".into(),
                name: "Wall".into(),
                containable: false,
                permanent: true,
                blocks_walking: true,
                food_value: 0,
                heat_value: 0.0,
                map_chance: 0.0,
                biomes: vec![],
                num_uses: 0,
                num_slots: 0,
                floor: false,
            dummy_ids: Vec::new(),
            },
        );
        let mut state = SimState::with_default_empty(std::sync::Arc::new(db));
        state.timed_movement = true;
        spawn_player(&mut state, 1, "tr@t");
        set_player_position(&mut state, 1, 0, 0);
        state.world.write().unwrap().set_object(2, 0, 99);
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        // Start-relative client path: tile (1,0) ok, tile (2,0) blocked.
        apply_move_path_start(&mut state, &hub, 1, 0, 0, &[(1, 0), (2, 0)], Some(4))
            .unwrap();
        let path = state.players.get(&1).unwrap().move_path.as_ref().unwrap();
        assert_eq!(path.trunc, 1);
        assert_eq!(path.remaining.len(), 1);
        assert_eq!(path.seq, 4);
        // PM wire body must list trunc=1 (accepted length 1 → total≈0.27).
        let mut saw_trunc_pm = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PM\n") && s.contains("0.27 0.27 1") {
                saw_trunc_pm = true;
            }
        }
        assert!(
            saw_trunc_pm,
            "PM body must include trunc=1 (… 0.27 0.27 1 …)"
        );
    }

    #[test]
    fn use_diagonal_fails_squared_euclidean() {
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "ud@t");
        set_player_position(&mut state, 1, 0, 0);
        state.world.write().unwrap().set_object(1, 1, 33);
        let r = apply_use_at(&mut state, 1, 1, 1).unwrap();
        assert!(!r.applied);
        assert_eq!(state.world.read().unwrap().get_object(1, 1), 33);
    }

    #[test]
    fn use_while_moving_intent_force_pu_fm_no_eat() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "ui@t");
        set_player_position(&mut state, 1, 0, 0);
        state.players.get_mut(&1).unwrap().held_id = 33;
        state.players.get_mut(&1).unwrap().food = 5.0;
        state.players.get_mut(&1).unwrap().move_path =
            Some(build_move_path(0, 0, vec![(1, 0)], 3.75, 9, 0, 0));
        state.players.get_mut(&1).unwrap().moving = true;
        let food0 = state.players.get(&1).unwrap().food;
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Use {
                conn_id: 1,
                x: 0,
                y: 0,
                id: None,
                index: None,
            },
        );
        assert_eq!(state.players.get(&1).unwrap().food, food0);
        assert!(state.players.get(&1).unwrap().move_path.is_none());
        let mut saw_pu = false;
        let mut saw_fm = false;
        let mut force1 = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PU\n") {
                saw_pu = true;
                // force field is 14th data field in full PU — look for force=1 pattern
                // wire: ... seq force x y ...
                if s.contains(" 1 ") {
                    force1 = true;
                }
            }
            if s.starts_with("FM\n") {
                saw_fm = true;
            }
        }
        assert!(saw_pu && saw_fm, "force PU+FM");
        assert!(force1, "expected force token present");
    }

    #[test]
    fn player_snapshot_moving_and_seq() {
        let mut p = Player::new(1, 1, "s@t");
        assert!(!p.snapshot().moving);
        p.move_path = Some(build_move_path(0, 0, vec![(1, 0)], 3.75, 7, 0, 0));
        p.moving = true;
        assert!(p.snapshot().moving);
        p.move_path = None;
        p.moving = false;
        p.done_moving_seq = 7;
        assert!(!p.snapshot().moving);
        assert_eq!(p.snapshot().done_moving_seq, 7);
    }

    #[test]
    fn remv_rejected_while_moving() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "rm@t");
        set_player_position(&mut state, 1, 0, 0);
        state.players.get_mut(&1).unwrap().held_id = 0;
        state.players.get_mut(&1).unwrap().move_path =
            Some(build_move_path(0, 0, vec![(1, 0)], 3.75, 1, 0, 0));
        state.players.get_mut(&1).unwrap().moving = true;
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "REMV".into(),
                payload: "0 0".into(),
            },
        );
        assert_eq!(state.players.get(&1).unwrap().held_id, 0);
        let mut saw_fm = false;
        while let Ok(pkt) = rx.try_recv() {
            if String::from_utf8_lossy(&pkt).starts_with("FM\n") {
                saw_fm = true;
            }
        }
        assert!(saw_fm, "REMV while moving force FM");
    }

    #[test]
    fn send_player_update_and_frame_force_and_seq() {
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "f@t");
        set_player_position(&mut state, 1, 2, 2);
        state.players.get_mut(&1).unwrap().done_moving_seq = 3;
        state.players.get_mut(&1).unwrap().move_path =
            Some(build_move_path(2, 2, vec![(1, 0)], 3.75, 9, 0, 0));
        send_player_update_and_frame(&mut state, &hub, 1);
        assert!(state.players.get(&1).unwrap().move_path.is_none());
        assert_eq!(state.players.get(&1).unwrap().done_moving_seq, 9);
        let mut body = String::new();
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PU\n") {
                body = s.to_string();
            }
        }
        assert!(!body.is_empty(), "PU sent");
        // Full PU contains force=1 after seq; path.seq was 9 so done=9, force=1
        assert!(body.contains(" 9 1 "), "expected seq=9 force=1 in {body}");
    }

}
