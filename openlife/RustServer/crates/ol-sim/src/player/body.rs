//! Live player body owned by the sim (`Player` / clothing / backpack).
//!
//! Index of all player-related files: [`super`] (`player/mod.rs`).

/// Pure multi-use numberOfUses helpers (Haxe TransitionHelper **TH-MULTI**).
#[path = "../multi_use.rs"]
pub mod multi_use;

use crate::move_path::MovePath;
use ol_world::NestedHelper;
use serde::Serialize;
use std::collections::VecDeque;

/// Haxe `clothingObjects` length (`Vector(6)`).
pub const CLOTHING_SLOT_COUNT: usize = 6;

/// Map Rust [`ClothingSlot`] → Haxe clothingObjects index.
/// 0 hat, 1 chest/tunic, 2 shoes (primary), 3 right shoe, 4 bottom, 5 backpack.
pub fn clothing_slot_index(slot: ClothingSlot) -> usize {
    match slot {
        ClothingSlot::Hat => 0,
        ClothingSlot::Chest => 1,
        ClothingSlot::Shoes => 2,
    }
}

/// Max object ids storable in [`Player::backpack`] (SAY STORE / TAKE / INV).
pub const BACKPACK_MAX: usize = 8;

/// Max personal journal notes on a player (`SAY NOTE` / `?NOTES` / `REMEMBER` / `?MEMORY`).
pub const NOTES_MAX: usize = 5;

/// Max characters stored per note (excess is truncated).
pub const NOTE_TEXT_MAX: usize = 80;

/// Max characters for personal title (`SAY TITLE`; excess is truncated).
pub const TITLE_TEXT_MAX: usize = 40;

/// Wearable body slots (object id; 0 = empty).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClothingSlot {
    Hat,
    Chest,
    Shoes,
}

impl ClothingSlot {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Hat => "hat",
            Self::Chest => "chest",
            Self::Shoes => "shoes",
        }
    }

    /// Parse slot name (`hat` / `chest` / `shoes`), case-insensitive.
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "hat" => Some(Self::Hat),
            "chest" => Some(Self::Chest),
            "shoes" | "shoe" => Some(Self::Shoes),
            _ => None,
        }
    }
}

/// Infer clothing slot from Haxe `clothing` field only (`h`/`t`/`s`).
///
/// `"Stone Hatchet"` contains `"hat"` but `clothing=n` is **not** wearable.
/// Name heuristics are not used (Haxe `ObjectData.getClothingSlot`).
// Haxe: ObjectData.getClothingSlot L1541–1561
pub fn clothing_slot_for_object(name: &str, description: &str) -> Option<ClothingSlot> {
    clothing_slot_for_object_ex(name, description, "")
}

/// Like [`clothing_slot_for_object`] plus explicit `clothing` (`h`/`t`/`s`/`b`/`p`/`n`).
pub fn clothing_slot_for_object_ex(
    _name: &str,
    description: &str,
    clothing: &str,
) -> Option<ClothingSlot> {
    let from_field = clothing.trim().chars().next().map(|ch| ch.to_ascii_lowercase());
    let from_desc = description
        .split("clothing=")
        .nth(1)
        .and_then(|rest| rest.chars().next())
        .map(|ch| ch.to_ascii_lowercase());
    let c = from_field.filter(|ch| *ch != 'n').or(from_desc);
    match c {
        Some('h') => Some(ClothingSlot::Hat),
        Some('t') => Some(ClothingSlot::Chest),
        Some('s') => Some(ClothingSlot::Shoes),
        _ => None,
    }
}

#[derive(Debug, Clone)]
pub struct Player {
    pub p_id: i32,
    pub conn_id: u64,
    pub email: String,
    /// LOGIN `client_tag` (Haxe `isOpenLifeClient` = contains `OpenLifeClientName`).
    // Haxe: Connection.login client_tag / isOpenLifeClient
    pub client_tag: String,
    pub x: i32,
    pub y: i32,
    /// Absolute world birth origin (Haxe `gx`/`gy`, vanilla `birthPos`).
    /// Client wire coords are relative: `world = client + birth`.
    pub birth_x: i32,
    pub birth_y: i32,
    pub held_id: i32,
    /// Haxe `heldObject.numberOfUses` — multi-use count for held (0 = N/A / single).
    pub held_uses: i32,
    /// Haxe `heldObject` full nest tree (contained backpacks, multi-use meta, timers).
    /// Mirrors [`Self::held_id`] / [`Self::held_uses`] when set; `None` ⇔ empty hands.
    /// // Haxe: GlobalPlayerInstance.heldObject
    pub held_helper: Option<NestedHelper>,
    pub food: f32,
    pub food_max: f32,
    pub age: f32,
    pub deleted: bool,
    pub connected: bool,
    /// Haxe `Connection.serverAi != null` — AI drives body after human disconnect.
    /// Cleared on rlogin reclaim or death (human-replacement AIs do not rebirth).
    // Haxe: Connection.close → new ServerAi(player); AI-TAKEOVER
    pub ai_controlled: bool,
    pub moving: bool,
    pub death_reason: Option<String>,
    /// Prefer last-use transition table on next USE (Haxe multi-use last).
    pub force_last_use: bool,
    /// Haxe `done_moving_seqNum` — >0 means stationary; set to path.seq on complete/cancel.
    pub done_moving_seq: i32,
    /// Active timed path (Haxe `newMoves`); `Some` ⇔ [`Self::moving`].
    pub move_path: Option<MovePath>,
    /// Center of last MAP_CHUNK sent (Haxe sendMapChunkIfNeeded).
    pub last_mc_x: i32,
    pub last_mc_y: i32,
    /// Whether any MC was sent for this life.
    pub has_mc: bool,
    /// Sim-seconds when last MAP_CHUNK was sent (Haxe `timeLastMapChunkSend`).
    // Haxe: Connection.timeLastMapChunkSend / MaxTimeBetweenMapChunks = 3
    pub last_mc_sim: f32,
    pub tools: crate::tools::ToolSlots,
    pub yum: crate::yum::YumState,
    /// Display first/last name for NM packet.
    pub first_name: String,
    pub family_name: String,
    /// Person object id on the wire (`po_id` in PU) — skin/body. Default 19.
    pub display_object_id: i32,
    /// Accumulates sim time toward BW/DY emit while starving infant (age&lt;3, food&lt;5).
    pub vitals_emit_timer: f32,
    /// Accumulates sim time toward PE hunger emote while food&lt;3.
    pub hunger_emot_timer: f32,
    /// Accumulates sim time toward PE sleep/snore emote while [`Self::sleeping`].
    pub sleep_emot_timer: f32,
    /// Haxe `lastTimeEmoteSend` — sim_time stamp for ambient UpdateEmotes 9s gate.
    // Haxe: GlobalPlayerInstance.lastTimeEmoteSend / TimeHelper.UpdateEmotes
    // FEVER-EMOTE
    pub last_time_emote_send: f32,
    /// Personal home tile (SAY HOME / GOHOME).
    pub home_x: i32,
    pub home_y: i32,
    /// Clothing slots: object ids, 0 = empty. (Wire / query surface — 3 of 6 Haxe slots.)
    pub hat: i32,
    pub chest: i32,
    pub shoes: i32,
    /// Haxe `clothingObjects` — full NestedHelper per slot (0..5).
    /// Indices 0–2 mirror hat/chest/shoes; 3–5 are right-shoe / bottom / backpack.
    /// // Haxe: GlobalPlayerInstance.clothingObjects
    pub clothing_helpers: [Option<NestedHelper>; CLOTHING_SLOT_COUNT],
    /// Haxe `hiddenWound` — light wound held invisibly (not droppable as cargo).
    /// // Haxe: GlobalPlayerInstance.hiddenWound
    pub hidden_wound: Option<NestedHelper>,
    /// Haxe `fever` — yellow-fever timer object (often wound id 2155).
    /// // Haxe: GlobalPlayerInstance.fever
    pub fever: Option<NestedHelper>,
    /// Haxe `yellowfeverCount` — resistance / infection accumulator.
    pub yellowfever_count: f32,
    /// True after SAY SLEEP until SAY WAKE; blocks MOVE and halves food drain.
    pub sleeping: bool,
    /// True after SAY SIT until SAY STAND; blocks MOVE and mildly reduces food drain.
    pub sitting: bool,
    /// True after SAY SICK until SAY CURE; multiplies food drain and sets DY isSick.
    pub sick: bool,
    /// True after SAY RIDE / MOUNT until SAY DISMOUNT; move-speed is noted only (no MOVE change).
    pub riding: bool,
    /// Godmode flag (SAY GODMODE). Enables lite god edits (`SAY VOGSET`); no invuln yet.
    pub godmode: bool,
    /// When true (`SAY DEAF` toggle), ignore normal/shout/mumble chat PS; whispers still deliver.
    pub deaf: bool,
    /// Haxe `displayBiomeAnimal` — one-shot “im save here from …” on biome miss.
    // Haxe: GlobalPlayerInstance.displayBiomeAnimal (not saved)
    // MOSQUITO-SAY-SAVE
    pub display_biome_animal: bool,
    /// Sim-time timestamps of recent SAY intents (sliding window rate limit).
    pub last_say_times: VecDeque<f32>,
    /// PE / EMOTE rate limit (separate from SAY; max 3 / 10s).
    pub emote_rate: crate::emote_limit::EmoteRateLimiter,
    /// Pending coin trade offer: `(target_p_id, amount)` from SAY TRADE; cleared on ACCEPT.
    pub trade_offer: Option<(i32, i32)>,
    /// Personal item storage (SAY STORE / TAKE / INV); max [`BACKPACK_MAX`].
    pub backpack: Vec<i32>,
    /// Personal journal lines (`SAY NOTE` / `?NOTES` / `REMEMBER` / `?MEMORY`); max [`NOTES_MAX`].
    pub notes: Vec<String>,
    /// Optional personal title (`SAY TITLE`); shown in `SAY ?NAME` when non-empty.
    pub title: String,
    /// Baby `p_id` currently held (0 = none). Haxe baby-carrying.
    pub holding_player_id: i32,
    /// Mother's `p_id` when this player is held as a baby (0 = none).
    pub held_by: i32,
    /// Sim-time of last successful lite profession action
    /// (`HARVEST` / `FISH` / `MINE` / `DIG` / `CHOP`).
    /// Initialized negative so the first action is not on cooldown.
    pub last_prof_action_time: f32,
    /// Haxe `GlobalPlayerInstance.owning` — absolute world tiles of owned objects
    /// (filled by `InitObjectHelpersAfterRead` / CLAIM / DROP ownership).
    pub owning: Vec<(i32, i32)>,
    /// Haxe body heat `0..1` (0.5 ideal). Updated from tile ambient in vitals.
    /// MAP-TEMP-PLAYER / `updateTemperature`.
    pub heat: f32,
    /// Haxe `GlobalPlayerInstance.storedWater` — cooling reserve from water drink.
    // Haxe: GlobalPlayerInstance.storedWater (doSelf drink)
    pub stored_water: f32,
    /// Haxe `warmPlace` remembered hot tile (jungle/desert fitness).
    // Haxe: GlobalPlayerInstance.warmPlace
    pub warm_place: Option<(i32, i32)>,
    /// Haxe `coldPlace` remembered cold tile (snow/water fitness).
    // Haxe: GlobalPlayerInstance.coldPlace
    pub cold_place: Option<(i32, i32)>,
    /// Last ambient temperature used for heat step (HX / debug).
    pub last_temperature: f32,
    /// Haxe `foodUsePerSecond` — temperature-scaled base drain (HEAT foodDrainTime).
    /// Written in `tick_vitals` from GPI `updateTemperature`; persist PLB1.
    // Haxe: GlobalPlayerInstance.foodUsePerSecond
    pub food_use_per_second: f32,
    /// Haxe `GlobalPlayerInstance.isCursed` — bone-grave proximity curse.
    /// S-MOVE-LIVE-GATES / MoveHelper.calculateSpeed.
    pub is_cursed: bool,
    /// Haxe `GlobalPlayerInstance.angryTime` — combat anger; <0 enables
    /// close-enemy-with-weapon speed mali (S-MOVE-LIVE-GATES).
    pub angry_time: f32,
    /// Haxe `GlobalPlayerInstance.killMode` — set by KILL; cleared on USE.
    /// Drains angryTime toward `CombatAngryTimeMinimum` while true.
    // Haxe: GlobalPlayerInstance.killMode
    pub kill_mode: bool,
    /// Haxe `GlobalPlayerInstance.darkNosaj` — dark-minion curse flag (session only; not saved).
    /// `> 0` doubles takeCoins wound factor (cap 1) and blocks combat-reputation restore.
    // Haxe: GlobalPlayerInstance.darkNosaj (not saved)
    // WALLET-COINS
    pub dark_nosaj: f32,
    /// Haxe `GlobalPlayerInstance.praisedJinbali` — Tarr praise flag (session; not saved).
    // Haxe: GlobalPlayerInstance.praisedJinbali
    // DARK-NOSAJ
    pub praised_jinbali: bool,
    /// Haxe `lastAttackedPlayer` p_id (0 = none) — breaks isFriendly with target.
    // Haxe: GlobalPlayerInstance.lastAttackedPlayer
    pub last_attacked_player_id: i32,
    /// Haxe `lastPlayerAttackedMe` p_id (0 = none) — breaks isFriendly with attacker.
    // Haxe: GlobalPlayerInstance.lastPlayerAttackedMe
    pub last_player_attacked_me_id: i32,
    /// Sticky AI smith profession stage / last / assigned (Haxe `profession['SMITH']`).
    /// Survives ticks; live ladder uses [`crate::SmithProfessionRuntime`] via
    /// `try_decide_smith_from_rung` / `smith_goal_from_map_and_rung` (AI-JOB-SMITH-WIRE).
    // Haxe: AiBase.profession['SMITH'] + lastProfession / assignedProfession
    pub smith_profession: crate::SmithProfessionRuntime,
    /// Sticky AI baker profession stage / last / assigned / last_pie / count_pies
    /// (Haxe `profession['BAKER']` + `lastPie` + `countPies`).
    /// Survives ticks; live ladder uses [`crate::BakerProfessionRuntime`] via
    /// `try_decide_baker_from_rung` / `baker_goal_from_map_and_rung` (AI-JOB-BAKER-WIRE).
    // Haxe: AiBase.profession['BAKER'] + lastProfession / assignedProfession + lastPie/countPies
    pub baker_profession: crate::BakerProfessionRuntime,
    /// Sticky AI potter profession stage / last / assigned (Haxe `profession['POTTER']`).
    // Haxe: AiBase.profession['POTTER'] + lastProfession / assignedProfession
    pub pottery_profession: crate::PotterProfessionRuntime,
    /// Sticky AI shepherd profession last / assigned / weight (Haxe `profession['SHEPHERD']`).
    // Haxe: AiBase.profession['SHEPHERD'] + lastProfession / assignedProfession
    pub shepherd_profession: crate::ShepherdProfessionRuntime,
    /// Sticky AI fire-food maker last / assigned / weight (Haxe `profession['FIREFOODMAKER']`).
    // Haxe: AiBase.profession['FIREFOODMAKER'] + lastProfession (AI-MAKE-STUFF)
    pub fire_food_profession: crate::FireFoodProfessionRuntime,
    /// Sticky AI fire keeper (Haxe profession['FIREKEEPER']) (AI-HANDLING-FIRE).
    // Haxe: AiBase.profession['FIREKEEPER'] + lastProfession / assignedProfession
    pub fire_keeper_profession: crate::FireKeeperProfessionRuntime,
    /// Sticky AI grave keeper (Haxe profession['GRAVEKEEPER']) (AI-JOB-GRAVE).
    // Haxe: AiBase.profession['GRAVEKEEPER'] + lastProfession / assignedProfession / lastGrave
    pub grave_keeper_profession: crate::GraveKeeperProfessionRuntime,
    /// Sticky AI hunter (Haxe profession['HUNTER']) (AI-JOB-HUNT).
    // Haxe: AiBase.profession['HUNTER'] + lastProfession / assignedProfession
    pub hunter_profession: crate::HunterProfessionRuntime,
    /// Sticky AI lumberjack (Haxe profession['LUMBERJACK']) (AI-JOB-LUMBER).
    // Haxe: AiBase.profession['LUMBERJACK'] + lastProfession / assignedProfession
    pub lumberjack_profession: crate::LumberjackProfessionRuntime,
    /// Sticky AI collector (Haxe profession['COLLECTOR']) (AI-JOB-COLLECT).
    // Haxe: AiBase.profession['COLLECTOR'] + lastProfession / assignedProfession
    pub collector_profession: crate::CollectorProfessionRuntime,
    /// Sticky AI food server (Haxe profession['FOODSERVER']) (AI-JOB-FOODSERVER).
    // Haxe: AiBase.profession['FOODSERVER'] + lastProfession / assignedProfession / feedingPlayerTarget
    pub foodserver_profession: crate::FoodServerProfessionRuntime,
    /// Sticky baker task hysteresis (`makeRawPies`, kindling, plant flags).
    // Haxe: AiBase.taskState makeRawPies / makeOrCollect / doPlant*
    pub baker_task: crate::BakerTaskState,
    /// Sticky AI farm profession last/assigned + weights (Haxe farm profession keys).
    /// Survives ticks; live ladder uses [`crate::FarmProfessionRuntime`] via
    /// `try_decide_farm_from_rung` / `farm_goal_from_map_and_rung` (AI-JOB-FARM-LIVE).
    // Haxe: AiBase.profession BASICFARMER/… + lastProfession / assignedProfession
    pub farm_profession: crate::FarmProfessionRuntime,
    /// Sticky farm task hysteresis (soil/row/plant/water/compost flags).
    // Haxe: AiBase.taskState SoilMaker / RowMaker / CornPlanter / …
    pub farm_task: crate::FarmTaskState,
    /// Sticky multi-tick craft state (Haxe itemToCraft + failedCraftings + itemToCraftId).
    /// Survives AI ticks; pure craft_item decisions mutate via [`crate::PlayerCraftAi`].
    // Haxe: AiBase.itemToCraft + failedCraftings + itemToCraftId + craftingTasks (AI-CRAFT-STICKY)
    pub craft_ai: crate::PlayerCraftAi,
    /// Sticky LLM speech cooldown + chunked SAY queue (Haxe `timeReactedLastCommand`).
    // Haxe: AiBase.timeReactedLastCommand / setWaitingTimeMin (AI-LLM-WIRE)
    pub llm_speech: crate::LlmSpeechRuntime,
    /// Haxe `GlobalPlayerInstance.exhaustion` — lockpick / fatigue / jump accumulator.
    // Haxe: GlobalPlayerInstance.exhaustion (TH-LOCK LockPick / MoveHelper jump)
    pub exhaustion: f32,
    /// Haxe `jumpedTiles` — sliding jump budget (decays × MaxJumpsPerTenSec×0.1 / s).
    // Haxe: GlobalPlayerInstance.jumpedTiles
    pub jumped_tiles: f32,
    /// Haxe `moveHelper.waitForForce` — ignore human MOVE until force ack / ~2s timeout.
    // Haxe: MoveHelper.waitForForce
    pub wait_for_force: bool,
    /// Haxe `moveHelper.timeLastForce` — `sim_time` when wait_for_force was set.
    // Haxe: MoveHelper.timeLastForce
    pub time_last_force: f32,
    /// Haxe `forceStopOnNextTile` — AI/path cancel after next tile commit.
    // Haxe: GlobalPlayerInstance.forceStopOnNextTile
    pub force_stop_on_next_tile: bool,
    /// Haxe `GlobalPlayerInstance.playerSoul` — AI interaction + chat memory.
    /// Lazily created in Haxe; always present (empty) in Rust (**AI-SOUL-WIRE**).
    // Haxe: GlobalPlayerInstance._playerSoul / playerSoul
    pub soul: crate::PlayerSoul,
    /// Haxe `GlobalPlayerInstance.partner` — mate `p_id` (0 = none). Not persisted yet.
    // Haxe: GlobalPlayerInstance.partner
    pub partner_p_id: i32,
    /// Haxe `GlobalPlayerInstance.trueAge` — wall-clock life years for soul prompts / death UI.
    /// Distinct from [`Self::age`] when health/starvation lag display age (Haxe sets age=trueAge on death).
    // Haxe: GlobalPlayerInstance.trueAge
    pub true_age: f32,
    /// Haxe `GlobalPlayerInstance.age_r` — seconds per display year (`AgeingSecondsPerYear / ageingFactor`).
    /// Updated each vitals tick from [`crate::food_store_max::age_step_from_health`].
    // Haxe: GlobalPlayerInstance.age_r / TimeHelper.updateAge L725
    pub age_r: f32,
    /// Haxe `assignedProfession` free string for AI NPCs (speech / job ladder).
    /// When set, overrides sticky smith/baker/farm assembly in soul prompts.
    // Haxe: GlobalPlayerInstance.assignedProfession
    pub assigned_profession: Option<String>,
    /// Haxe `lastProfession` free string for AI NPCs.
    // Haxe: GlobalPlayerInstance.lastProfession
    pub last_profession: Option<String>,
    /// Haxe `newFollower` — p_id of player requesting to join (0 = none).
    /// Session pending; delayed confirm via live `TimeConfirmNewFollower` (default 15s).
    /// Haxe hire uses immediate `setFollowPlayer` — does not use this slot.
    // FOLLOW-HIRE-DELAY
    // Haxe: GlobalPlayerInstance.newFollower
    pub new_follower_id: i32,
    /// Haxe `newFollowerFor` — direct follow target p_id for the pending request.
    // Haxe: GlobalPlayerInstance.newFollowerFor
    pub new_follower_for_id: i32,
    /// Haxe `newFollowerTime` — countdown seconds until pending follow confirms.
    // Haxe: GlobalPlayerInstance.newFollowerTime
    pub new_follower_time: f32,
    /// Haxe `allowShowHuman` — false after attacking a human; blocks `!H` map pin.
    /// Session-only (not saved).
    // Haxe: GlobalPlayerInstance.allowShowHuman
    // MAP-LOCATION-PINS
    pub allow_show_human: bool,
    /// Haxe `AiBase.notReachableObjects` + `objectsWithHostilePath` (PATH-REACH).
    /// Session AI path-block timers; cleaned in `tick_vitals`.
    // Haxe: AiBase L85–86 notReachableObjects / objectsWithHostilePath
    pub ai_path_reach: crate::ai_path_reach::AiPathReachMaps,
    /// Sticky AI food/use/drop/block claims for live CalculateBlockedByAi (**BLOCKED-BY-AI**).
    // Haxe: AiBase.foodTarget / dropTarget / useTarget + GPI.blockTargetForAi
    pub ai_block_targets: crate::ai_path_reach::AiStickyBlockTargets,
    /// Haxe `AiBase.playerToFollow` p_id (0 = none). LLM + scripted FOLLOW.
    /// Not leadership social.following — AI walk-with target (**AI-LLM-APPLY**).
    // Haxe: AiBase.playerToFollow
    pub ai_follow_p_id: i32,
    /// Haxe `AiBase.autoStopFollow` — true = loose follow / auto clear.
    // Haxe: AiBase.autoStopFollow
    pub ai_auto_stop_follow: bool,
    /// Haxe `AiBase.timeStartedToFolow` as sim_time when ordered follow began.
    // Haxe: AiBase.timeStartedToFolow
    pub ai_follow_started_sim_time: f32,
    /// Haxe `AiBase.orderedToDrop` — next AI tick dropHeldObject(0).
    // Haxe: AiBase.orderedToDrop
    pub ai_ordered_to_drop: bool,
    /// Haxe `AiBase.debugSay` (AI-SAY-HELPER DEBUG ON/OFF).
    // Haxe: AiBase.debugSay
    pub ai_debug_say: bool,
    /// Haxe `AiBase.debugProfession` (PROF ON/OFF).
    // Haxe: AiBase.debugProfession
    pub ai_debug_profession: bool,
    /// Haxe `AiBase.isNiceBaby` (NICE? reply).
    // Haxe: AiBase.isNiceBaby
    pub ai_is_nice_baby: bool,
    /// Haxe `GlobalPlayerInstance.firePlace` sticky tile (0 id = none).
    /// Updated by HOME! / GetCloseFire after SearchNewHome.
    // Haxe: GlobalPlayerInstance.firePlace / AiHelper.GetCloseFire
    pub ai_fire_place_id: i32,
    pub ai_fire_place_x: i32,
    pub ai_fire_place_y: i32,
    /// Haxe `AiBase.isHandlingTemperature` (AI-HANDLE-TEMP).
    // Haxe: AiBase.isHandlingTemperature
    pub ai_handling_temperature: bool,
    /// Haxe `AiBase.justArrived` while handling temperature.
    // Haxe: AiBase.justArrived
    pub ai_temp_just_arrived: bool,
    /// Haxe `AiBase.lastTemperature` (previous body heat).
    // Haxe: AiBase.lastTemperature
    pub ai_last_heat: f32,
    /// Haxe `AiBase.lastGotoObj` parent id (0 = none). Used by gotoObj receding abort.
    // Haxe: AiBase.lastGotoObj / AiHelper.gotoObj ~1086
    // AI-GOTO-FOOD
    pub ai_last_goto_obj_id: i32,
    pub ai_last_goto_obj_x: i32,
    pub ai_last_goto_obj_y: i32,
    /// Haxe `AiBase.lastGotoObjDistance` (quad distance).
    // Haxe: AiBase.lastGotoObjDistance
    pub ai_last_goto_obj_distance: f32,
    /// Haxe `AiBase.didNotReachFood` — gates considerAnimals + escape.
    // Haxe: AiBase.didNotReachFood
    pub ai_did_not_reach_food: f32,
    /// Haxe `AiBase.foodTarget` sticky tile (0 id = none).
    // Haxe: AiBase.foodTarget
    pub ai_sticky_food_id: i32,
    pub ai_sticky_food_x: i32,
    pub ai_sticky_food_y: i32,
    /// Haxe `AiBase.timeLastLeaderCheck` (allyUp 10s cadence).
    // Haxe: AiBase.allyUp timeLastLeaderCheck (AI-ALLY-UP-HIRE)
    pub ai_last_leader_check_sim: f32,
    /// Haxe `blockedTeleportLocations` — linear map indexes already tried by
    /// `!TCG`/`!TV`/`teleport` closest-pick cycle (session-only).
    // Haxe: GlobalPlayerInstance.blockedTeleportLocations
    // CURSED-GRAVE-TELEPORT
    pub blocked_teleport_locations: Vec<i32>,
}

impl Player {
    pub fn new(p_id: i32, conn_id: u64, email: &str) -> Self {
        // Placeholders; `spawn_player` assigns via `naming::pick_random_name`.
        Self {
            p_id,
            conn_id,
            email: email.to_string(),
            client_tag: String::new(),
            x: 0,
            y: 0,
            birth_x: 0,
            birth_y: 0,
            held_id: 0,
            held_uses: 0,
            held_helper: None,
            food: 10.0,
            food_max: 20.0,
            age: 14.0,
            deleted: false,
            connected: true,
            ai_controlled: false,
            moving: false,
            death_reason: None,
            force_last_use: false,
            done_moving_seq: 1,
            move_path: None,
            last_mc_x: 0,
            last_mc_y: 0,
            has_mc: false,
            last_mc_sim: 0.0,
            tools: crate::tools::ToolSlots::default(),
            yum: crate::yum::YumState::default(),
            first_name: "NEWBORN".into(),
            family_name: "SNOW".into(),
            display_object_id: 19,
            vitals_emit_timer: 0.0,
            hunger_emot_timer: 0.0,
            sleep_emot_timer: 0.0,
            last_time_emote_send: 0.0,
            home_x: 0,
            home_y: 0,
            hat: 0,
            chest: 0,
            shoes: 0,
            clothing_helpers: [None, None, None, None, None, None],
            hidden_wound: None,
            fever: None,
            yellowfever_count: 0.0,
            sleeping: false,
            sitting: false,
            sick: false,
            riding: false,
            godmode: false,
            deaf: false,
            display_biome_animal: true,
            last_say_times: VecDeque::new(),
            emote_rate: crate::emote_limit::EmoteRateLimiter::default(),
            trade_offer: None,
            backpack: Vec::new(),
            notes: Vec::new(),
            title: String::new(),
            holding_player_id: 0,
            held_by: 0,
            last_prof_action_time: -crate::professions::PROF_ACTION_COOLDOWN_SECS,
            owning: Vec::new(),
            heat: 0.5,
            stored_water: 0.0,
            warm_place: None,
            cold_place: None,
            last_temperature: 0.5,
            food_use_per_second: crate::FOOD_USE_PER_SEC,
            is_cursed: false,
            angry_time: crate::move_live_gates::COMBAT_ANGRY_TIME_BEFORE_ATTACK,
            kill_mode: false,
            // WALLET-COINS: darkNosaj session-only (Haxe not saved)
            dark_nosaj: 0.0,
            // DARK-NOSAJ: praisedJinbali session flag
            praised_jinbali: false,
            last_attacked_player_id: 0,
            last_player_attacked_me_id: 0,
            smith_profession: crate::SmithProfessionRuntime::default(),
            baker_profession: crate::BakerProfessionRuntime::default(),
            pottery_profession: crate::PotterProfessionRuntime::default(),
            shepherd_profession: crate::ShepherdProfessionRuntime::default(),
            fire_food_profession: crate::FireFoodProfessionRuntime::default(),
            fire_keeper_profession: crate::FireKeeperProfessionRuntime::default(),
            grave_keeper_profession: crate::GraveKeeperProfessionRuntime::default(),
            hunter_profession: crate::HunterProfessionRuntime::default(),
            lumberjack_profession: crate::LumberjackProfessionRuntime::default(),
            collector_profession: crate::CollectorProfessionRuntime::default(),
            foodserver_profession: crate::FoodServerProfessionRuntime::default(),
            baker_task: crate::BakerTaskState::default(),
            farm_profession: crate::FarmProfessionRuntime::default(),
            farm_task: crate::FarmTaskState::default(),
            craft_ai: crate::PlayerCraftAi::new(),
            llm_speech: crate::LlmSpeechRuntime::default(),
            exhaustion: 0.0,
            jumped_tiles: 0.0,
            wait_for_force: false,
            time_last_force: 0.0,
            force_stop_on_next_tile: false,
            soul: crate::PlayerSoul::new(),
            partner_p_id: 0,
            true_age: 14.0,
            // Haxe: age_r = AgeingSecondsPerYear at init
            age_r: crate::food_store_max::AGEING_SECONDS_PER_YEAR,
            assigned_profession: None,
            last_profession: None,
            new_follower_id: 0,
            new_follower_for_id: 0,
            new_follower_time: 0.0,
            allow_show_human: true,
            // PATH-REACH: empty notReachable / hostile path maps
            ai_path_reach: crate::ai_path_reach::AiPathReachMaps::default(),
            // BLOCKED-BY-AI: sticky food/use/drop/block claims
            ai_block_targets: crate::ai_path_reach::AiStickyBlockTargets::default(),
            // AI-LLM-APPLY: playerToFollow / orderedToDrop sticky
            ai_follow_p_id: 0,
            ai_auto_stop_follow: true,
            ai_follow_started_sim_time: 0.0,
            ai_ordered_to_drop: false,
            // AI-SAY-HELPER: debugSay / debugProfession / isNiceBaby
            ai_debug_say: false,
            ai_debug_profession: false,
            ai_is_nice_baby: true,
            // AI-SAY-HELPER: firePlace sticky (HOME! GetCloseFire)
            ai_fire_place_id: 0,
            ai_fire_place_x: 0,
            ai_fire_place_y: 0,
            ai_handling_temperature: false,
            ai_temp_just_arrived: false,
            ai_last_heat: 0.5,
            // AI-GOTO-FOOD: lastGotoObj + didNotReachFood
            ai_last_goto_obj_id: 0,
            ai_last_goto_obj_x: 0,
            ai_last_goto_obj_y: 0,
            ai_last_goto_obj_distance: -1.0,
            ai_did_not_reach_food: 0.0,
            ai_sticky_food_id: 0,
            ai_sticky_food_x: 0,
            ai_sticky_food_y: 0,
            ai_last_leader_check_sim: 0.0,
            // CURSED-GRAVE-TELEPORT: blockedTeleportLocations session list
            blocked_teleport_locations: Vec::new(),
        }
    }

    /// Whether this player has free hands to pick up a baby/child.
    ///
    /// Haxe `doBabyHelper` only requires empty hands (and not already holding a
    /// player). Age gates are separate: carrier ≥ target+1 and target &lt; 10
    /// ([`crate::feed::can_pickup_player_ages`]) — **no hard age≥14**.
    // Haxe: doBabyHelper L4947-4964
    pub fn can_hold_baby(&self) -> bool {
        crate::feed::can_hold_baby_hands(self.deleted, self.holding_player_id, self.held_id)
    }

    /// Begin holding baby `baby_p_id` (caller must also set the baby's `held_by`).
    ///
    /// Haxe `o_id = [-heldPlayer.p_id]` so PU `held` is negative (client in-arms).
    // Haxe: GPI.SetTransitionData / toData heldObject
    pub fn start_holding(&mut self, baby_p_id: i32) {
        self.holding_player_id = baby_p_id;
        self.held_id = -baby_p_id;
        self.held_uses = 0;
        self.held_helper = None;
    }

    /// Release held baby; returns baby `p_id` or 0 if none.
    pub fn release_holding(&mut self) -> i32 {
        let id = self.holding_player_id;
        self.holding_player_id = 0;
        if self.held_id < 0 {
            self.clear_held();
        }
        id
    }

    /// Clear held object and multi-use counter (does not clear [`Self::hidden_wound`]).
    pub fn clear_held(&mut self) {
        self.held_id = 0;
        self.held_uses = 0;
        self.held_helper = None;
    }

    /// Set held object + multi-use count (flat; nest cleared unless id matches existing helper).
    ///
    /// Prefer [`Self::set_held_helper`] when nest/timers matter.
    // Haxe: GlobalPlayerInstance.setHeldObject (id/uses subset)
    pub fn set_held(&mut self, id: i32, uses: i32) {
        if id == 0 {
            self.clear_held();
            return;
        }
        let uses = uses.max(0);
        self.held_id = id;
        self.held_uses = uses;
        match &mut self.held_helper {
            Some(h) if h.id == id => {
                h.uses_remaining = uses;
            }
            _ => {
                self.held_helper = Some(NestedHelper::with_uses(id, uses));
            }
        }
    }

    /// Set held object from a NestedHelper tree (id/uses mirrored to flat fields).
    // Haxe: GlobalPlayerInstance.setHeldObject (full ObjectHelper)
    pub fn set_held_helper(&mut self, helper: NestedHelper) {
        let id = helper.id;
        let uses = helper.uses_remaining;
        if id == 0 {
            self.clear_held();
            return;
        }
        self.held_id = id;
        self.held_uses = uses.max(0);
        self.held_helper = Some(helper);
    }

    /// True when held object is the light/hidden wound alias.
    pub fn is_holding_hidden_wound(&self) -> bool {
        match (&self.held_helper, &self.hidden_wound) {
            (Some(h), Some(w)) => h.id == w.id && h.id != 0,
            _ => false,
        }
    }

    /// Heavy (non-hidden) wound held — grave / AI wound flag.
    ///
    /// Light wounds alias to `hidden_wound` and return false even when `is_wound`.
    pub fn is_wounded_held(&self, is_wound: bool) -> bool {
        if !is_wound || self.held_id == 0 {
            return false;
        }
        !self.is_holding_hidden_wound()
    }

    /// Flat clothing id for a 3-slot surface.
    pub fn clothing(&self, slot: ClothingSlot) -> i32 {
        match slot {
            ClothingSlot::Hat => self.hat,
            ClothingSlot::Chest => self.chest,
            ClothingSlot::Shoes => self.shoes,
        }
    }

    /// Set flat clothing id and keep `clothing_helpers` in sync (flat id only).
    pub fn set_clothing(&mut self, slot: ClothingSlot, id: i32) {
        let id = id.max(0);
        match slot {
            ClothingSlot::Hat => self.hat = id,
            ClothingSlot::Chest => self.chest = id,
            ClothingSlot::Shoes => self.shoes = id,
        }
        let idx = clothing_slot_index(slot);
        if id == 0 {
            self.clothing_helpers[idx] = None;
        } else {
            match &mut self.clothing_helpers[idx] {
                Some(h) if h.id == id => {}
                _ => {
                    self.clothing_helpers[idx] = Some(NestedHelper::with_uses(id, 0));
                }
            }
        }
    }

    /// NestedHelper for a 3-slot surface (indices 0..2).
    pub fn clothing_helper(&self, slot: ClothingSlot) -> Option<&NestedHelper> {
        self.clothing_helpers[clothing_slot_index(slot)].as_ref()
    }

    /// Set NestedHelper on a 3-slot surface; mirrors flat hat/chest/shoes.
    pub fn set_clothing_helper(&mut self, slot: ClothingSlot, helper: NestedHelper) {
        let idx = clothing_slot_index(slot);
        let id = helper.id;
        if id == 0 {
            self.clothing_helpers[idx] = None;
            self.set_flat_clothing_id(slot, 0);
            return;
        }
        self.set_flat_clothing_id(slot, id);
        self.clothing_helpers[idx] = Some(helper);
    }

    /// Set NestedHelper by Haxe clothingObjects index 0..5.
    // Haxe: clothingObjects[i] =
    pub fn set_clothing_index_helper(&mut self, index: usize, helper: Option<NestedHelper>) {
        if index >= CLOTHING_SLOT_COUNT {
            return;
        }
        let id = helper
            .as_ref()
            .map(|h| h.id)
            .filter(|&id| id > 0)
            .unwrap_or(0);
        self.clothing_helpers[index] = if id == 0 { None } else { helper };
        match index {
            0 => self.hat = id,
            1 => self.chest = id,
            2 => self.shoes = id,
            5 => {
                if id == 0 {
                    self.backpack.clear();
                } else {
                    self.sync_flat_backpack_from_nest();
                }
            }
            _ => {}
        }
    }

    /// Haxe `getPackpack()` — clothingObjects[5] when worn (`id > 0`).
    // Haxe: GlobalPlayerInstance.getPackpack
    pub fn worn_backpack(&self) -> Option<&NestedHelper> {
        self.clothing_helpers[5].as_ref().filter(|h| h.id > 0)
    }

    /// Copy nest contained ids into flat [`Self::backpack`] (worn pack only).
    // BACKPACK-NEST-DUAL
    pub fn sync_flat_backpack_from_nest(&mut self) {
        if let Some(pack) = self.clothing_helpers[5].as_ref().filter(|h| h.id > 0) {
            self.backpack = pack
                .contained
                .iter()
                .map(|c| c.id)
                .filter(|&id| id > 0)
                .collect();
        }
    }

    /// Cargo ids: worn pack nest contained, else flat [`Self::backpack`].
    // Haxe: getPackpack().containedObjects
    // BACKPACK-NEST-DUAL
    pub fn backpack_cargo_ids(&self) -> Vec<i32> {
        if let Some(pack) = self.worn_backpack() {
            pack.contained
                .iter()
                .map(|c| c.id)
                .filter(|&id| id > 0)
                .collect()
        } else {
            self.backpack.clone()
        }
    }

    /// Cargo count for INV / WEIGHT / ballast (nest when a pack is worn).
    // BACKPACK-NEST-DUAL
    pub fn backpack_cargo_len(&self) -> usize {
        match self.worn_backpack() {
            Some(pack) => pack.contained.iter().filter(|c| c.id > 0).count(),
            None => self.backpack.len(),
        }
    }

    fn set_flat_clothing_id(&mut self, slot: ClothingSlot, id: i32) {
        match slot {
            ClothingSlot::Hat => self.hat = id,
            ClothingSlot::Chest => self.chest = id,
            ClothingSlot::Shoes => self.shoes = id,
        }
    }

    /// Equip held into `slot`, swapping previous clothing into hands.
    ///
    /// Returns `(equipped_id, previous_slot_id)`.
    // Haxe: doSwitchCloths subset for hat/chest/shoes
    pub fn wear_held(&mut self, slot: ClothingSlot) -> Result<(i32, i32), &'static str> {
        if self.held_id == 0 {
            return Err("EMPTY");
        }
        let idx = clothing_slot_index(slot);
        let held = match self.held_helper.take() {
            Some(h) if h.id == self.held_id || self.held_id == 0 => h,
            _ => NestedHelper::with_uses(self.held_id, self.held_uses),
        };
        let held_id = held.id;
        let prev = self.clothing_helpers[idx].take();
        let prev_id = prev
            .as_ref()
            .map(|h| h.id)
            .filter(|&id| id > 0)
            .unwrap_or_else(|| self.clothing(slot));
        self.clothing_helpers[idx] = Some(held);
        self.set_flat_clothing_id(slot, held_id);
        if prev_id > 0 {
            let prev_h = prev
                .filter(|p| !p.is_empty() && p.id > 0)
                .unwrap_or_else(|| NestedHelper::with_uses(prev_id, 0));
            self.set_held_helper(prev_h);
        } else {
            self.clear_held();
        }
        Ok((held_id, prev_id))
    }

    /// Strip clothing slot into empty hands.
    // Haxe: remove clothing into hands
    pub fn strip_slot(&mut self, slot: ClothingSlot) -> Result<i32, &'static str> {
        if self.held_id != 0 {
            return Err("HANDS");
        }
        let idx = clothing_slot_index(slot);
        let prev = self.clothing_helpers[idx].take();
        let prev_id = prev
            .as_ref()
            .map(|h| h.id)
            .filter(|&id| id > 0)
            .unwrap_or(0);
        if prev_id == 0 {
            // Flat may still hold id when helper missing
            let flat = self.clothing(slot);
            self.set_flat_clothing_id(slot, 0);
            if flat == 0 {
                return Err("EMPTY");
            }
            self.set_held(flat, 0);
            return Ok(flat);
        }
        self.set_flat_clothing_id(slot, 0);
        if let Some(p) = prev {
            self.set_held_helper(p);
        }
        Ok(prev_id)
    }

    /// Move held object into personal backpack (`SAY STORE`).
    ///
    /// When clothing slot 5 is a worn pack, also push into nest contained
    /// (Haxe `clothingObjects[5].containedObjects`) and sync flat ids.
    // BACKPACK-NEST-DUAL
    pub fn store_to_backpack(&mut self) -> Result<i32, &'static str> {
        if self.held_id == 0 {
            return Err("EMPTY");
        }
        let nest_len = self
            .clothing_helpers[5]
            .as_ref()
            .filter(|h| h.id > 0)
            .map(|h| h.contained.len());
        if nest_len.unwrap_or(self.backpack.len()) >= BACKPACK_MAX {
            return Err("FULL");
        }
        let id = self.held_id;
        let uses = self.held_uses;
        let helper = self
            .held_helper
            .take()
            .unwrap_or_else(|| NestedHelper::with_uses(id, uses));
        if let Some(pack) = self.clothing_helpers[5].as_mut().filter(|h| h.id > 0) {
            pack.contained.push(helper);
            self.clear_held();
            self.sync_flat_backpack_from_nest();
            return Ok(id);
        }
        self.backpack.push(id);
        self.clear_held();
        Ok(id)
    }

    /// Move backpack index into empty hands (`SAY TAKE`).
    ///
    /// Worn pack: remove from nest contained, then sync flat ids.
    // BACKPACK-NEST-DUAL
    pub fn take_from_backpack(&mut self, index: usize) -> Result<i32, &'static str> {
        if self.held_id != 0 {
            return Err("HANDS");
        }
        if self.clothing_helpers[5]
            .as_ref()
            .filter(|h| h.id > 0)
            .is_some()
        {
            let pack = self.clothing_helpers[5].as_mut().unwrap();
            if index >= pack.contained.len() {
                return Err("BAD");
            }
            let taken = pack.contained.remove(index);
            let id = taken.id;
            self.set_held_helper(taken);
            self.sync_flat_backpack_from_nest();
            return Ok(id);
        }
        if index >= self.backpack.len() {
            return Err("BAD");
        }
        let id = self.backpack.remove(index);
        self.set_held(id, 0);
        Ok(id)
    }

    /// `SAY INV` body without leading p_id: `INV n/max id1 id2…` or `INV 0/max`.
    pub fn inv_report(&self) -> String {
        let ids = self.backpack_cargo_ids();
        let n = ids.len();
        if n == 0 {
            format!("INV 0/{BACKPACK_MAX}")
        } else {
            let s: Vec<String> = ids.iter().map(|id| id.to_string()).collect();
            format!("INV {n}/{BACKPACK_MAX} {}", s.join(" "))
        }
    }

    /// `SAY CLOTHES` body: `CLOTHES hat=N chest=N shoes=N`.
    pub fn clothes_report(&self) -> String {
        format!(
            "CLOTHES hat={} chest={} shoes={}",
            self.hat, self.chest, self.shoes
        )
    }

    /// `SAY NOTES` body: `NOTES n/max` or with lines `0:text; 1:text…`.
    pub fn notes_report(&self) -> String {
        let n = self.notes.len();
        if n == 0 {
            return format!("NOTES 0/{NOTES_MAX}");
        }
        let parts: Vec<String> = self
            .notes
            .iter()
            .enumerate()
            .map(|(i, t)| format!("{i}:{t}"))
            .collect();
        format!("NOTES {n}/{NOTES_MAX} {}", parts.join("; "))
    }

    /// Append a personal journal note (`SAY NOTE` / `REMEMBER`).
    pub fn add_note(&mut self, text: &str) -> Result<(), &'static str> {
        let t = text.trim();
        if t.is_empty() {
            return Err("EMPTY");
        }
        if self.notes.len() >= NOTES_MAX {
            return Err("FULL");
        }
        let mut s = t.to_string();
        if s.chars().count() > NOTE_TEXT_MAX {
            s = s.chars().take(NOTE_TEXT_MAX).collect();
        }
        self.notes.push(s);
        Ok(())
    }

    /// Pop last journal note (`SAY FORGET`).
    pub fn pop_note(&mut self) -> Result<String, &'static str> {
        self.notes.pop().ok_or("EMPTY")
    }

    /// Set personal title (`SAY TITLE`); empty clears.
    pub fn set_title(&mut self, text: &str) -> Result<&str, &'static str> {
        let t = text.trim();
        if t.is_empty() {
            return Err("EMPTY");
        }
        let mut s = t.to_string();
        if s.chars().count() > TITLE_TEXT_MAX {
            s = s.chars().take(TITLE_TEXT_MAX).collect();
        }
        self.title = s;
        Ok(self.title.as_str())
    }

    /// Drain held + clothing + backpack for death scatter (clears all).
    pub fn take_death_loot_for_scatter(&mut self) -> Vec<i32> {
        let mut items = Vec::new();
        if self.held_id != 0 && !self.is_holding_hidden_wound() {
            items.push(self.held_id);
        }
        self.clear_held();
        for slot in [ClothingSlot::Hat, ClothingSlot::Chest, ClothingSlot::Shoes] {
            let id = self.clothing(slot);
            if id != 0 {
                items.push(id);
            }
            self.set_clothing(slot, 0);
        }
        // Extended clothing slots 3..4 (right shoe / bottom)
        for i in 3..5 {
            if let Some(h) = self.clothing_helpers[i].take() {
                if h.id > 0 {
                    items.push(h.id);
                }
            }
        }
        // Slot 5: nest cargo then bag (do not also append dual-written flat ids).
        // BACKPACK-NEST-DUAL
        if let Some(h) = self.clothing_helpers[5].take() {
            for c in h.contained {
                if c.id > 0 {
                    items.push(c.id);
                }
            }
            if h.id > 0 {
                items.push(h.id);
            }
            self.backpack.clear();
        } else {
            items.append(&mut self.backpack);
        }
        items
    }

    /// Drain held + backpack cargo for DROPALL (keeps clothing / worn pack).
    pub fn take_dropall_for_scatter(&mut self) -> Vec<i32> {
        let mut items = Vec::new();
        if self.held_id != 0 && !self.is_holding_hidden_wound() {
            items.push(self.held_id);
        }
        self.clear_held();
        if let Some(pack) = self.clothing_helpers[5].as_mut().filter(|h| h.id > 0) {
            for c in pack.contained.drain(..) {
                if c.id > 0 {
                    items.push(c.id);
                }
            }
            self.backpack.clear();
        } else {
            items.append(&mut self.backpack);
        }
        items
    }

    /// Display name `"First Family"`.
    pub fn display_name(&self) -> String {
        format!("{} {}", self.first_name, self.family_name)
    }

    /// Viewer role tag derived from account identity without exposing the email.
    pub fn public_role(&self) -> String {
        public_agent_role(&self.email, self.is_ai_body())
    }

    /// Name for SAY / chat bubbles (display + optional title).
    // Haxe: name used in say / PS lines
    pub fn name_for_say(&self) -> String {
        if self.title.is_empty() {
            self.display_name()
        } else {
            format!("{} ({})", self.display_name(), self.title)
        }
    }

    /// Name for `?NAME` query (same as display; title optional).
    pub fn name_for_query(&self) -> String {
        if self.title.is_empty() {
            self.display_name()
        } else {
            format!("{} — {}", self.display_name(), self.title)
        }
    }

    /// Set birth origin for client-relative wire coords (Haxe birthPos).
    // Haxe: birth_x / birth_y origin for PU/MC relative coords
    pub fn set_birth_origin(&mut self, world_x: i32, world_y: i32) {
        self.birth_x = world_x;
        self.birth_y = world_y;
    }

    /// World → birth-relative coords (plain subtract, no torus wrap).
    /// PU/PM player tiles use Haxe `transformX/Y` (`viewer_pu_xy`) instead.
    #[inline]
    pub fn world_to_client(&self, world_x: i32, world_y: i32) -> (i32, i32) {
        (world_x - self.birth_x, world_y - self.birth_y)
    }

    /// Client-relative → world coords (add birth origin).
    #[inline]
    pub fn client_to_world(&self, client_x: i32, client_y: i32) -> (i32, i32) {
        (client_x + self.birth_x, client_y + self.birth_y)
    }

    /// Haxe `moveHelper.exactTx/exactTy` — sub-tile while pathing, else tile.
    // Haxe: MoveHelper.exactTx / exactTy
    #[inline]
    pub fn exact_xy(&self) -> (f64, f64) {
        match &self.move_path {
            Some(path) => (path.exact_x as f64, path.exact_y as f64),
            None => (self.x as f64, self.y as f64),
        }
    }

    /// True when MAP_CHUNK should resend (never sent, or Chebyshev move ≥ threshold).
    // Haxe: sendMapChunkIfNeeded distance gate
    pub fn needs_map_chunk(&self, threshold: i32) -> bool {
        if !self.has_mc {
            return true;
        }
        let dx = (self.x - self.last_mc_x).abs();
        let dy = (self.y - self.last_mc_y).abs();
        dx.max(dy) >= threshold
    }

    /// Haxe `isAi()` for this body (takeover / offline / permanent AI email).
    // Haxe: Connection.isAi / GlobalPlayerInstance.isAi (AI-TAKEOVER)
    pub fn is_ai_body(&self) -> bool {
        crate::ai_takeover::player_is_ai(self.connected, self.ai_controlled, &self.email)
    }

    /// Haxe `isHuman()`.
    pub fn is_human_body(&self) -> bool {
        crate::ai_takeover::player_is_human(self.connected, self.ai_controlled, &self.email)
    }

    /// Flat clothing parent ids for all 6 Haxe clothingObjects slots.
    // Haxe: clothingObjects[i].parentId (DROP-HELD-TABLE quiver snapshot)
    pub fn clothing_parent_ids(&self) -> [i32; 6] {
        let mut ids = [0i32; 6];
        for i in 0..CLOTHING_SLOT_COUNT {
            ids[i] = self.clothing_helpers[i]
                .as_ref()
                .map(|h| h.id)
                .filter(|&id| id > 0)
                .unwrap_or(0);
        }
        if ids[0] == 0 {
            ids[0] = self.hat.max(0);
        }
        if ids[1] == 0 {
            ids[1] = self.chest.max(0);
        }
        if ids[2] == 0 {
            ids[2] = self.shoes.max(0);
        }
        ids
    }

    /// Multi-use remaining per clothing slot (quiver capacity).
    // Haxe: clothingObjects[i].numberOfUses
    pub fn clothing_uses_remaining(&self) -> [i32; 6] {
        let mut uses = [0i32; 6];
        for i in 0..CLOTHING_SLOT_COUNT {
            uses[i] = self.clothing_helpers[i]
                .as_ref()
                .map(|h| h.uses_remaining.max(0))
                .unwrap_or(0);
        }
        uses
    }

    /// Snapshot for web / self-play UI.
    /// Haxe `AiBase.newBorn` craft wipe (failedCraftings + itemToCraft + tasks).
    // Haxe: AiBase.newBorn (AI-CRAFT-STICKY)
    pub fn wipe_craft_on_birth(&mut self) {
        self.craft_ai.wipe_on_birth();
    }

    /// Haxe `calledCraftItem = false` each AI doTime entry.
    // Haxe: AiBase.doTimeStuffHelper calledCraftItem = false
    pub fn craft_ai_begin_tick(&mut self) {
        self.craft_ai.begin_tick();
    }

    pub fn snapshot(&self) -> PlayerSnapshot {
        PlayerSnapshot {
            conn_id: self.conn_id,
            p_id: self.p_id,
            x: self.x,
            y: self.y,
            held_id: self.held_id,
            held_uses: self.held_uses,
            // NPC-SCAN-FULL: held nest cargo for gatherClay / dropHeldObject Basket+Clay
            // Haxe: heldObject.containedObjects.length / contains([126])
            held_contained: crate::short_craft_intent::profession_scan::held_contained_from_player(
                self,
            ),
            held_contains_clay:
                crate::short_craft_intent::profession_scan::held_contains_clay_from_player(self),
            food: self.food,
            food_max: self.food_max,
            age: self.age,
            hits: 0.0,
            sick: self.sick,
            email: self.email.clone(),
            is_ai: self.is_ai_body(),
            display_name: self.display_name(),
            role: public_agent_role(&self.email, self.is_ai_body()),
            deleted: self.deleted,
            connected: self.connected,
            ai_controlled: self.ai_controlled,
            moving: self.moving || self.move_path.is_some(),
            done_moving_seq: self.done_moving_seq,
            heat: self.heat,
            last_temperature: self.last_temperature,
            warm_place: self.warm_place,
            cold_place: self.cold_place,
            held_by: self.held_by,
            // DROP-HELD-LIVE: mother holding baby (Haxe getHeldPlayer / dropPlayer)
            holding_player_id: self.holding_player_id,
            clothing: self.clothing_parent_ids(),
            clothing_uses: self.clothing_uses_remaining(),
            // AI-FOLLOW-WALK: sticky walk-with for NPC continuous follow
            ai_follow_p_id: self.ai_follow_p_id,
            ai_auto_stop_follow: self.ai_auto_stop_follow,
            // PATH-REACH-MERGE: dual-map pull source for npc_ai
            ai_path_reach: self.ai_path_reach.clone(),
            // AI-JOB-SMITH-RESID: home + profession sticky for NPC peer count
            // Haxe: GlobalPlayerInstance.home.tx/ty; lastProfession == 'SMITH'|…
            home_x: self.home_x,
            home_y: self.home_y,
            is_last_smith: self.smith_profession.is_last_smith,
            is_last_baker: self.baker_profession.is_last_baker,
            is_last_potter: self.pottery_profession.is_last_potter,
            is_last_shepherd: self.shepherd_profession.is_last_shepherd,
            is_last_farm: self.farm_profession.last_profession.is_some(),
            last_farm: self.farm_profession.last_profession,
            is_last_fire_food: self.fire_food_profession.is_last_fire_food,
            is_last_hunter: self.hunter_profession.is_last_hunter
                || self.last_profession.as_deref() == Some("HUNTER"),
            is_last_lumberjack: self.lumberjack_profession.is_last_lumberjack
                || self.last_profession.as_deref() == Some("LUMBERJACK"),
            is_last_collector: self.collector_profession.is_last_collector
                || self.last_profession.as_deref() == Some("COLLECTOR"),
            is_last_foodserver: self.foodserver_profession.is_last_foodserver
                || self.last_profession.as_deref() == Some("FOODSERVER"),
            is_last_tailor: self.last_profession.as_deref() == Some("TAILOR"),
            is_assigned_tailor: self.assigned_profession.as_deref() == Some("TAILOR"),
            ai_fire_place_id: self.ai_fire_place_id,
            ai_fire_place_x: self.ai_fire_place_x,
            ai_fire_place_y: self.ai_fire_place_y,
            // AI-PRIO-LIVE: GetCloseDeadlyPlayer / isAnimalDeadlyForMe / clothing sex
            angry_time: self.angry_time,
            is_cursed: self.is_cursed,
            display_object_id: self.display_object_id,
            last_attacked_player_id: self.last_attacked_player_id,
            last_player_attacked_me_id: self.last_player_attacked_me_id,
            ai_did_not_reach_food: self.ai_did_not_reach_food,
            ai_last_goto_obj_id: self.ai_last_goto_obj_id,
            ai_last_goto_obj_x: self.ai_last_goto_obj_x,
            ai_last_goto_obj_y: self.ai_last_goto_obj_y,
            ai_last_goto_obj_distance: self.ai_last_goto_obj_distance,
            ai_sticky_food_id: self.ai_sticky_food_id,
            ai_sticky_food_x: self.ai_sticky_food_x,
            ai_sticky_food_y: self.ai_sticky_food_y,
            is_hidden_wound: self.is_holding_hidden_wound(),
            hidden_wound_id: self.hidden_wound.as_ref().map(|h| h.id).unwrap_or(0),
            lost_combat_prestige: 0.0,
        }
    }
}

/// Viewer role tag from identity heuristics. Never returns the email itself.
pub fn public_agent_role(email: &str, is_ai: bool) -> String {
    let e = email.to_ascii_lowercase();
    if e.contains("farmer") {
        "farmer".into()
    } else if e.contains("hunter") {
        "hunter".into()
    } else if e.contains("selfplay") {
        "forager".into()
    } else if is_ai {
        "ai".into()
    } else {
        "player".into()
    }
}

/// Read-only view for web viewer / self-play UI (updated by sim after mutations).
#[derive(Debug, Clone, Serialize)]
pub struct PlayerSnapshot {
    pub conn_id: u64,
    pub p_id: i32,
    pub x: i32,
    pub y: i32,
    pub held_id: i32,
    /// Multi-use count for held (0 = N/A).
    #[serde(default)]
    pub held_uses: i32,
    /// Haxe `heldObject.containedObjects.length` (basket clay cargo).
    // Haxe: heldObject.containedObjects.length gatherClay
    #[serde(default)]
    pub held_contained: i32,
    /// Haxe `heldObject.contains([126])` or held is bare clay.
    // Haxe: dropHeldObject Basket 292 + Clay 126
    #[serde(default)]
    pub held_contains_clay: bool,
    pub food: f32,
    pub food_max: f32,
    pub age: f32,
    /// Haxe `hits` (combat). isFeedingChild drops when fed and hits &lt; 1.
    #[serde(default)]
    pub hits: f32,
    /// Haxe `isIll()` / SAY SICK.
    #[serde(default)]
    pub sick: bool,
    /// Identity key — never serialized to web/JSON (contact address is the only public email).
    #[serde(skip_serializing)]
    pub email: String,
    /// Server-side AI heuristic (permanent AI / takeover). Safe for viewer JSON.
    #[serde(default)]
    pub is_ai: bool,
    /// `"First Family"` display name for viewer / players page.
    #[serde(default)]
    pub display_name: String,
    /// Viewer role tag (`farmer`/`hunter`/`forager`/`ai`/`player`) — never an email.
    #[serde(default)]
    pub role: String,
    pub deleted: bool,
    /// Human TCP still attached (false after disconnect / AI takeover).
    #[serde(default = "default_true_snapshot")]
    pub connected: bool,
    /// AI drives this body after human disconnect (AI-TAKEOVER).
    #[serde(default)]
    pub ai_controlled: bool,
    /// True while a timed MovePath is active (or legacy moving flag).
    pub moving: bool,
    /// Seq of last completed/cancelled path (client `@seq` when provided).
    pub done_moving_seq: i32,
    /// Body heat 0..1 for AI superbad-temp sensors (MAP-TEMP-PLAYER).
    #[serde(default = "default_heat_snapshot")]
    pub heat: f32,
    /// Haxe `GlobalPlayerInstance.lastTemperature` ambient (AI-HANDLE-TEMP fail-cool).
    #[serde(default = "default_heat_snapshot")]
    pub last_temperature: f32,
    /// Haxe `warmPlace` tile for handleTemperature.
    #[serde(default)]
    pub warm_place: Option<(i32, i32)>,
    /// Haxe `coldPlace` tile for handleTemperature.
    #[serde(default)]
    pub cold_place: Option<(i32, i32)>,
    /// Mother `p_id` when held as baby (0 = none); AI mother/follow sensors.
    #[serde(default)]
    pub held_by: i32,
    /// Haxe `heldPlayer.p_id` when carrying a baby (0 = none).
    // Haxe: GlobalPlayerInstance.heldPlayer; AiBase dropPlayer before eat
    #[serde(default)]
    pub holding_player_id: i32,
    /// Haxe `clothingObjects` parent ids (6 slots) for quiver / clothing AI.
    // Haxe: clothingObjects (DROP-HELD-TABLE)
    #[serde(default)]
    pub clothing: [i32; 6],
    /// Haxe clothing `numberOfUses` per slot (quiver multi-use capacity).
    #[serde(default)]
    pub clothing_uses: [i32; 6],
    /// Sticky AI walk-with target p_id (Haxe playerToFollow); 0 = none (**AI-FOLLOW-WALK**).
    // Haxe: AiBase.playerToFollow
    #[serde(default)]
    pub ai_follow_p_id: i32,
    /// Haxe autoStopFollow — loose follow when true.
    // Haxe: AiBase.autoStopFollow
    #[serde(default = "default_true_snapshot")]
    pub ai_auto_stop_follow: bool,
    /// PATH-REACH-MERGE: expose timed maps for NPC dual-map sync.
    /// Not serialized to web JSON (serde skip).
    // Haxe: AiBase L85–86 notReachableObjects / objectsWithHostilePath
    // PATH-REACH-MERGE / dual_map_merge
    #[serde(skip)]
    pub ai_path_reach: crate::ai_path_reach::AiPathReachMaps,
    /// Personal home tile for profession peer same-home filter (Haxe `home.tx/ty`).
    // Haxe: GlobalPlayerInstance.home; AiBase.countProfession same-home
    // AI-JOB-SMITH-RESID / PlayerSnapshot home
    #[serde(default)]
    pub home_x: i32,
    #[serde(default)]
    pub home_y: i32,
    /// Sticky last profession is SMITH (Haxe `lastProfession == 'SMITH'`).
    // Haxe: AiBase.myPlayer.lastProfession; countProfession hasProfession
    // AI-JOB-SMITH-RESID
    #[serde(default)]
    pub is_last_smith: bool,
    /// Sticky last profession is BAKER.
    // Haxe: lastProfession == 'BAKER'
    #[serde(default)]
    pub is_last_baker: bool,
    /// Sticky last profession is POTTER.
    // Haxe: lastProfession == 'POTTER'
    #[serde(default)]
    pub is_last_potter: bool,
    /// Sticky last profession is SHEPHERD.
    // Haxe: lastProfession == 'SHEPHERD'
    #[serde(default)]
    pub is_last_shepherd: bool,
    /// Sticky last is any farm profession key (BASICFARMER / BerryFarmer / …).
    // Haxe: lastProfession farm family for countProfession farm jobs
    #[serde(default)]
    pub is_last_farm: bool,
    /// Job-specific last farm key (Haxe `lastProfession` farm family).
    // Haxe: countProfession('WATERBRINGER'|…)
    #[serde(skip)]
    pub last_farm: Option<crate::FarmProfession>,
    /// Sticky last profession is FIREFOODMAKER.
    // Haxe: lastProfession == 'FIREFOODMAKER'
    #[serde(default)]
    pub is_last_fire_food: bool,
    /// Sticky last profession is HUNTER (AI-JOB-HUNT).
    // Haxe: lastProfession == 'HUNTER'
    #[serde(default)]
    pub is_last_hunter: bool,
    /// Sticky last profession is LUMBERJACK (AI-JOB-LUMBER).
    // Haxe: lastProfession == 'LUMBERJACK'
    #[serde(default)]
    pub is_last_lumberjack: bool,
    /// Sticky last profession is COLLECTOR (AI-JOB-COLLECT).
    // Haxe: lastProfession == 'COLLECTOR'
    #[serde(default)]
    pub is_last_collector: bool,
    /// Sticky last profession is FOODSERVER (AI-JOB-FOODSERVER).
    // Haxe: lastProfession == 'FOODSERVER'
    #[serde(default)]
    pub is_last_foodserver: bool,
    /// Sticky last profession is TAILOR (`fillUpQuiver` countProfession).
    // Haxe: lastProfession == 'TAILOR'
    #[serde(default)]
    pub is_last_tailor: bool,
    /// Haxe `assignedProfession == 'TAILOR'` (AI-JOB-TAILOR).
    #[serde(default)]
    pub is_assigned_tailor: bool,
    /// Haxe `GlobalPlayerInstance.firePlace` (FIRE-PLACE-STICKY).
    #[serde(default)]
    pub ai_fire_place_id: i32,
    #[serde(default)]
    pub ai_fire_place_x: i32,
    #[serde(default)]
    pub ai_fire_place_y: i32,
    /// Haxe `angryTime` — GetCloseDeadlyPlayer / escape gates.
    // Haxe: GlobalPlayerInstance.angryTime; AiHelper.GetCloseDeadlyPlayerHelper
    // AI-PRIO-LIVE
    #[serde(default)]
    pub angry_time: f32,
    /// Haxe `isCursed` — deadly-player scan treats cursed as dangerous.
    // Haxe: GlobalPlayerInstance.isCursed
    #[serde(default)]
    pub is_cursed: bool,
    /// Person object id (PU `po_id`) for clothing sex + loved-biome animal path.
    // Haxe: GlobalPlayerInstance.displayedObjectId / ObjectData.male
    #[serde(default = "default_display_object_id_snapshot")]
    pub display_object_id: i32,
    /// Last player this body attacked (breaks isFriendly).
    // Haxe: GlobalPlayerInstance.lastAttackedPlayer
    #[serde(default)]
    pub last_attacked_player_id: i32,
    /// Last player who attacked this body (breaks isFriendly).
    // Haxe: GlobalPlayerInstance.lastPlayerAttackedMe
    #[serde(default)]
    pub last_player_attacked_me_id: i32,
    /// Haxe `hiddenWound` alias — light wound does not count as `isWounded()`.
    // Haxe: GlobalPlayerInstance.isWounded; hiddenWound == heldObject
    #[serde(default)]
    pub is_hidden_wound: bool,
    /// Hidden-wound object id (0 = none).
    #[serde(default)]
    pub hidden_wound_id: i32,
    /// Haxe `lostCombatPrestige` — GetCloseDeadlyPlayer danger. Filled at view publish.
    // Haxe: GlobalPlayerInstance.lostCombatPrestige
    #[serde(default)]
    pub lost_combat_prestige: f32,
    /// Haxe `didNotReachFood` / `lastGotoObj` / `foodTarget` for NPC↔Player merge.
    #[serde(default)]
    pub ai_did_not_reach_food: f32,
    #[serde(default)]
    pub ai_last_goto_obj_id: i32,
    #[serde(default)]
    pub ai_last_goto_obj_x: i32,
    #[serde(default)]
    pub ai_last_goto_obj_y: i32,
    #[serde(default = "default_last_goto_dist")]
    pub ai_last_goto_obj_distance: f32,
    #[serde(default)]
    pub ai_sticky_food_id: i32,
    #[serde(default)]
    pub ai_sticky_food_x: i32,
    #[serde(default)]
    pub ai_sticky_food_y: i32,
}

fn default_last_goto_dist() -> f32 {
    -1.0
}

fn default_display_object_id_snapshot() -> i32 {
    19
}

fn default_heat_snapshot() -> f32 {
    0.5
}

fn default_true_snapshot() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_snapshot_json_omits_email() {
        let p = Player::new(1, 1, "secret@example.com");
        let snap = p.snapshot();
        assert_eq!(snap.email, "secret@example.com");
        let v = serde_json::to_value(&snap).expect("serialize snapshot");
        assert!(v.get("email").is_none(), "email must not appear in viewer JSON");
        let s = v.to_string();
        assert!(!s.contains("secret@example.com"));
        assert!(!s.contains('@'));
        assert_eq!(v.get("role").and_then(|x| x.as_str()), Some("player"));
        assert_eq!(v.get("is_ai").and_then(|x| x.as_bool()), Some(false));
        assert!(v.get("display_name").is_some());
    }

    #[test]
    fn held_uses_set_and_clear() {
        let mut p = Player::new(1, 1, "u@test");
        assert_eq!(p.held_uses, 0);
        p.set_held(253, 3);
        assert_eq!((p.held_id, p.held_uses), (253, 3));
        p.clear_held();
        assert_eq!((p.held_id, p.held_uses), (0, 0));
    }

    #[test]
    fn pottery_profession_sticky_defaults_and_survives() {
        // AI-POTTER: Player.pottery_profession sticky across ticks
        let mut p = Player::new(1, 1, "potter@test");
        assert!(!p.pottery_profession.is_last_potter);
        assert_eq!(p.pottery_profession.stage, 0.0);
        assert!(crate::assign_potter_from_speech(
            &mut p.pottery_profession,
            "POTTER!"
        ));
        assert!(p.pottery_profession.is_assigned_potter);
        assert!(p.pottery_profession.is_last_potter);
        p.pottery_profession.stage = 10.0;
        assert_eq!(p.pottery_profession.stage, 10.0);
        p.pottery_profession.wipe_on_eat(false);
        assert_eq!(p.pottery_profession.stage, 0.0);
        assert!(!p.pottery_profession.is_last_potter);
    }

    #[test]
    fn shepherd_profession_sticky_defaults_and_survives() {
        // AI-SHEPHERD: Player.shepherd_profession sticky across ticks
        let mut p = Player::new(1, 1, "shepherd@test");
        assert!(!p.shepherd_profession.is_last_shepherd);
        assert_eq!(p.shepherd_profession.weight, 0.0);
        assert!(crate::assign_shepherd_from_speech(
            &mut p.shepherd_profession,
            "SHEPHERD!"
        ));
        assert!(p.shepherd_profession.is_assigned_shepherd);
        assert!(p.shepherd_profession.is_last_shepherd);
        assert_eq!(p.shepherd_profession.weight, 1.0);
        p.shepherd_profession.clear_weight();
        assert_eq!(p.shepherd_profession.weight, 0.0);
        p.shepherd_profession.wipe_on_eat(false);
        assert!(!p.shepherd_profession.is_last_shepherd);
    }

    #[test]
    fn fire_food_profession_sticky_defaults_and_survives() {
        // AI-MAKE-STUFF: Player.fire_food_profession sticky across ticks
        let mut p = Player::new(1, 1, "firefood@test");
        assert!(!p.fire_food_profession.is_last_fire_food);
        assert_eq!(p.fire_food_profession.weight, 0.0);
        assert!(crate::assign_fire_food_from_speech(
            &mut p.fire_food_profession,
            "FIREFOOD!"
        ));
        assert!(p.fire_food_profession.is_assigned_fire_food);
        assert!(p.fire_food_profession.is_last_fire_food);
        assert_eq!(p.fire_food_profession.weight, 1.0);
        p.fire_food_profession.clear_weight();
        assert_eq!(p.fire_food_profession.weight, 0.0);
        p.fire_food_profession.wipe_on_eat(false);
        assert!(!p.fire_food_profession.is_last_fire_food);
    }

    #[test]
    fn fire_keeper_profession_sticky_defaults_and_survives() {
        // AI-HANDLING-FIRE: Player.fire_keeper_profession sticky
        let mut p = Player::new(1, 1, "firekeep@test");
        assert!(!p.fire_keeper_profession.is_last_fire_keeper);
        assert!(crate::assign_fire_keeper_from_speech(
            &mut p.fire_keeper_profession,
            "FIREKEEPER!"
        ));
        assert!(p.fire_keeper_profession.is_assigned_fire_keeper);
        assert!(p.fire_keeper_profession.is_last_fire_keeper);
        p.fire_keeper_profession.clear_weight();
        p.fire_keeper_profession.wipe_on_eat(false);
        assert!(!p.fire_keeper_profession.is_last_fire_keeper);
    }

    #[test]
    fn grave_keeper_profession_sticky_defaults_and_survives() {
        let mut p = Player::new(1, 1, "gravekeep@test");
        assert!(!p.grave_keeper_profession.is_last_grave_keeper);
        assert!(crate::assign_grave_keeper_from_speech(
            &mut p.grave_keeper_profession,
            "GRAVEKEEPER!"
        ));
        assert!(p.grave_keeper_profession.is_assigned_grave_keeper);
        assert!(p.grave_keeper_profession.is_last_grave_keeper);
        p.grave_keeper_profession.clear_weight();
        p.grave_keeper_profession.wipe_on_eat(false);
        assert!(!p.grave_keeper_profession.is_last_grave_keeper);
    }

    #[test]
    fn hunter_profession_sticky_defaults_and_survives() {
        let mut p = Player::new(1, 1, "hunter@test");
        assert!(!p.hunter_profession.is_last_hunter);
        assert!(crate::assign_hunter_from_speech(
            &mut p.hunter_profession,
            "HUNTER!"
        ));
        assert!(p.hunter_profession.is_assigned_hunter);
        assert!(p.hunter_profession.is_last_hunter);
        p.hunter_profession.clear_weight();
        p.hunter_profession.wipe_on_eat(false);
        assert!(!p.hunter_profession.is_last_hunter);
    }

    #[test]
    fn lumberjack_profession_sticky_defaults_and_survives() {
        let mut p = Player::new(1, 1, "lumber@test");
        assert!(!p.lumberjack_profession.is_last_lumberjack);
        assert!(crate::assign_lumberjack_from_speech(
            &mut p.lumberjack_profession,
            "LUMBERJACK!"
        ));
        assert!(p.lumberjack_profession.is_assigned_lumberjack);
        assert!(p.lumberjack_profession.is_last_lumberjack);
        p.lumberjack_profession.clear_weight();
        p.lumberjack_profession.wipe_on_eat(false);
        assert!(!p.lumberjack_profession.is_last_lumberjack);
    }

    #[test]
    fn collector_profession_sticky_defaults_and_survives() {
        let mut p = Player::new(1, 1, "collect@test");
        assert!(!p.collector_profession.is_last_collector);
        assert!(crate::assign_collector_from_speech(
            &mut p.collector_profession,
            "COLLECTOR!"
        ));
        assert!(p.collector_profession.is_assigned_collector);
        assert!(p.collector_profession.is_last_collector);
        p.collector_profession.clear_weight();
        p.collector_profession.wipe_on_eat(false);
        assert!(!p.collector_profession.is_last_collector);
    }

    #[test]
    fn foodserver_profession_sticky_defaults_and_survives() {
        let mut p = Player::new(1, 1, "food@test");
        assert!(!p.foodserver_profession.is_last_foodserver);
        assert!(crate::assign_foodserver_from_speech(
            &mut p.foodserver_profession,
            "FOODSERVER!"
        ));
        assert!(p.foodserver_profession.is_assigned_foodserver);
        assert!(p.foodserver_profession.is_last_foodserver);
        p.foodserver_profession.clear_weight();
        p.foodserver_profession.wipe_on_eat(false);
        assert!(!p.foodserver_profession.is_last_foodserver);
    }

    #[test]
    fn smith_profession_sticky_defaults_and_survives() {
        // AI-JOB-SMITH-WIRE: Player.smith_profession sticky across ticks
        let mut p = Player::new(1, 1, "smith@test");
        assert!(!p.smith_profession.is_last_smith);
        assert_eq!(p.smith_profession.stage, 0.0);
        assert!(crate::assign_smith_from_speech(
            &mut p.smith_profession,
            "SMITH!"
        ));
        assert!(p.smith_profession.is_assigned_smith);
        assert!(p.smith_profession.is_last_smith);
        p.smith_profession.stage = 5.0;
        let stage = p.smith_profession.stage;
        assert_eq!(stage, 5.0);
        crate::wipe_smith_on_eat(&mut p.smith_profession, false);
        assert_eq!(p.smith_profession.stage, 0.0);
        assert!(!p.smith_profession.is_last_smith);
    }

    #[test]
    fn smith_hungry_path_consider_food_wipes_profession() {
        // AI-JOB-SMITH-LIVE: isConsideringMakingFood → wipe_smith_on_eat
        let mut p = Player::new(1, 1, "smith-hungry@test");
        assert!(crate::assign_smith_from_speech(
            &mut p.smith_profession,
            "SMITH!"
        ));
        p.smith_profession.stage = 6.0;
        // Simulate hungry consider-food path (age ok, hungry, not FOODSERVER)
        assert!(crate::apply_consider_making_food_smith_wipe(
            &mut p.smith_profession,
            20.0,
            true,
            false,
            3.0,
            false,
        ));
        assert_eq!(p.smith_profession.stage, 0.0);
        assert!(!p.smith_profession.is_last_smith);
        // FOODSERVER keeps sticky last
        p.smith_profession.is_last_smith = true;
        p.smith_profession.stage = 4.0;
        assert!(crate::apply_consider_making_food_smith_wipe(
            &mut p.smith_profession,
            20.0,
            true,
            false,
            3.0,
            true,
        ));
        assert_eq!(p.smith_profession.stage, 0.0);
        assert!(p.smith_profession.is_last_smith);
    }

    #[test]
    fn baker_profession_sticky_defaults_and_survives() {
        // AI-JOB-BAKER-WIRE: Player.baker_profession + baker_task sticky across ticks
        let mut p = Player::new(1, 1, "baker@test");
        assert!(!p.baker_profession.is_last_baker);
        assert_eq!(p.baker_profession.stage, 0.0);
        assert_eq!(p.baker_profession.last_pie, -1);
        assert_eq!(p.baker_profession.count_pies, 0);
        assert_eq!(p.baker_task.make_raw_pies, 0.0);
        assert!(crate::assign_baker_from_speech(
            &mut p.baker_profession,
            "BAKER!"
        ));
        assert!(p.baker_profession.is_assigned_baker);
        assert!(p.baker_profession.is_last_baker);
        p.baker_profession.stage = 3.0;
        p.baker_profession.last_pie = 2;
        p.baker_profession.count_pies = 4;
        p.baker_task.make_raw_pies = 1.0;
        assert_eq!(p.baker_profession.stage, 3.0);
        assert_eq!(p.baker_profession.last_pie, 2);
        assert_eq!(p.baker_profession.count_pies, 4);
        assert_eq!(p.baker_task.make_raw_pies, 1.0);
        crate::note_raw_pie_crafted(&mut p.baker_profession, crate::RAW_PIES[0]);
        assert_eq!(p.baker_profession.count_pies, 5);
    }

    #[test]
    fn snapshot_is_last_tailor_from_last_profession() {
        let mut p = Player::new(1, 1, "t@test");
        assert!(!p.snapshot().is_last_tailor);
        p.last_profession = Some("TAILOR".into());
        assert!(p.snapshot().is_last_tailor);
        p.last_profession = Some("SMITH".into());
        assert!(!p.snapshot().is_last_tailor);
    }

    #[test]
    fn snapshot_held_contained_and_clay_from_nest() {
        // NPC-SCAN-FULL: nest cargo + clay-in-basket for npc dropHeld / pottery
        // Haxe: heldObject.containedObjects.length; contains([126]); Basket 292
        let mut p = Player::new(1, 1, "clay@test");
        let empty = p.snapshot();
        assert_eq!(empty.held_contained, 0);
        assert!(!empty.held_contains_clay);

        p.set_held(126, 1);
        let bare = p.snapshot();
        assert_eq!(bare.held_id, 126);
        assert_eq!(bare.held_contained, 0);
        assert!(bare.held_contains_clay);

        p.set_held_helper(NestedHelper::from_wire(292, &[33]));
        let no_clay = p.snapshot();
        assert_eq!(no_clay.held_id, 292);
        assert_eq!(no_clay.held_contained, 1);
        assert!(!no_clay.held_contains_clay);

        let mut basket = NestedHelper::with_uses(292, 1);
        basket.contained.push(NestedHelper::id_only(126));
        basket.contained.push(NestedHelper::id_only(33));
        p.set_held_helper(basket);
        let nest = p.snapshot();
        assert_eq!(nest.held_id, 292);
        assert_eq!(nest.held_contained, 2);
        assert!(nest.held_contains_clay);
    }

    #[test]
    fn snapshot_held_player_id_from_start_holding() {
        // DROP-HELD-LIVE: npc isPickingupFood dropPlayer uses snapshot heldPlayer
        let mut p = Player::new(1, 1, "mom@test");
        assert_eq!(p.snapshot().holding_player_id, 0);
        p.start_holding(42);
        assert_eq!(p.snapshot().holding_player_id, 42);
        assert_eq!(p.held_id, -42);
        p.holding_player_id = 7;
        assert_eq!(p.snapshot().holding_player_id, 7);
        assert_eq!(p.release_holding(), 7);
        assert_eq!(p.snapshot().holding_player_id, 0);
    }

    #[test]
    fn snapshot_includes_angry_curse_and_display_for_ai_sensors() {
        // AI-PRIO-LIVE: GetCloseDeadlyPlayer / loved-biome path / clothing sex
        let mut p = Player::new(1, 1, "npc@test");
        let s0 = p.snapshot();
        assert_eq!(s0.display_object_id, 19);
        assert_eq!(s0.angry_time, p.angry_time);
        assert!(!s0.is_cursed);
        p.angry_time = 2.5;
        p.is_cursed = true;
        p.display_object_id = 19;
        p.last_attacked_player_id = 8;
        p.last_player_attacked_me_id = 9;
        let s = p.snapshot();
        assert!((s.angry_time - 2.5).abs() < f32::EPSILON);
        assert!(s.is_cursed);
        assert_eq!(s.display_object_id, 19);
        assert_eq!(s.last_attacked_player_id, 8);
        assert_eq!(s.last_player_attacked_me_id, 9);
        assert!(!s.is_hidden_wound);
        assert_eq!(s.hidden_wound_id, 0);
        assert_eq!(s.lost_combat_prestige, 0.0);
    }

    #[test]
    fn farm_profession_sticky_defaults_and_survives() {
        // AI-JOB-FARM-LIVE: Player.farm_profession + farm_task sticky across ticks
        let mut p = Player::new(1, 1, "farm@test");
        assert!(p.farm_profession.last_profession.is_none());
        assert!(p.farm_profession.assigned_profession.is_none());
        assert!(p.farm_profession.weights.is_empty());
        assert_eq!(p.farm_task.soil_maker, 0.0);
        assert_eq!(p.farm_task.row_maker, 0.0);
        assert_eq!(p.farm_task.corn_planter, 0.0);

        assert!(!crate::assign_farm_from_speech(
            &mut p.farm_profession,
            "BAKER!"
        ));
        assert!(crate::assign_farm_from_speech(
            &mut p.farm_profession,
            "FARMER!"
        ));
        assert_eq!(
            p.farm_profession.assigned_profession,
            Some(crate::FarmProfession::BasicFarmer)
        );
        assert_eq!(
            p.farm_profession.last_profession,
            Some(crate::FarmProfession::BasicFarmer)
        );
        assert_eq!(
            p.farm_profession
                .weights
                .get(&crate::FarmProfession::BasicFarmer),
            Some(&1.0)
        );

        assert!(crate::assign_farm_from_speech(
            &mut p.farm_profession,
            "WHEAT!"
        ));
        assert_eq!(
            p.farm_profession.assigned_profession,
            Some(crate::FarmProfession::BasicFarmer)
        );
        assert!(crate::assign_farm_from_speech(
            &mut p.farm_profession,
            "CARROT!"
        ));
        assert_eq!(
            p.farm_profession.assigned_profession,
            Some(crate::FarmProfession::CarrotFarmer)
        );
        assert!(crate::assign_farm_from_speech(
            &mut p.farm_profession,
            "ROW!"
        ));
        assert_eq!(
            p.farm_profession.assigned_profession,
            Some(crate::FarmProfession::RowMaker)
        );
        assert!(crate::assign_farm_from_speech(
            &mut p.farm_profession,
            "SOIL!"
        ));
        assert_eq!(
            p.farm_profession.assigned_profession,
            Some(crate::FarmProfession::SoilMaker)
        );
        assert!(crate::assign_farm_from_speech(
            &mut p.farm_profession,
            "WATER!"
        ));
        assert_eq!(
            p.farm_profession.assigned_profession,
            Some(crate::FarmProfession::WaterBringer)
        );

        p.farm_task.soil_maker = 1.0;
        p.farm_task.row_maker = 2.0;
        p.farm_task.corn_planter = 1.0;
        // Simulate tick boundary: fields survive on the same Player
        assert_eq!(p.farm_task.soil_maker, 1.0);
        assert_eq!(p.farm_task.row_maker, 2.0);
        assert_eq!(p.farm_task.corn_planter, 1.0);
        assert_eq!(
            crate::resolve_farm_assigned_job(&p.farm_profession),
            Some(crate::FarmProfession::WaterBringer)
        );
    }

    #[test]
    fn ai_path_reach_sticky_defaults_and_survives() {
        // PATH-REACH: Player.ai_path_reach sticky across ticks
        let mut p = Player::new(1, 1, "path@test");
        assert!(p.ai_path_reach.is_empty());
        p.ai_path_reach.add_not_reachable(10, 20, 90.0);
        p.ai_path_reach.add_hostile_path(11, 21, 20.0);
        assert!(p.ai_path_reach.is_personal_not_reachable(10, 20));
        assert!(p.ai_path_reach.is_object_with_hostile_path(11, 21));
        p.ai_path_reach.cleanup(100.0);
        assert!(p.ai_path_reach.is_empty());
    }

    #[test]
    fn craft_ai_sticky_on_player_defaults_and_survives() {
        // AI-CRAFT-STICKY: Player.craft_ai sticky across ticks
        let mut p = Player::new(1, 1, "craft@test");
        assert_eq!(p.craft_ai.item_to_craft_id, -1);
        assert!(p.craft_ai.crafting_tasks.is_empty());
        assert_eq!(p.craft_ai.runtime.last_actor_id, -1);
        assert!(p.craft_ai.item_to_craft_name.is_none());
        p.craft_ai.item_to_craft_id = 83;
        p.craft_ai.item_to_craft_name = Some("Fire".into());
        p.craft_ai.add_task(71, true);
        p.craft_ai.runtime.failed.record_fail(83, 50.0);
        p.craft_ai.runtime.item = crate::ItemToCraftState::new(83);
        p.craft_ai.runtime.item.count_done = 1;
        // Survive "tick boundary" (same Player)
        assert_eq!(p.craft_ai.item_to_craft_id, 83);
        assert_eq!(p.craft_ai.crafting_tasks, vec![71]);
        assert!(p.craft_ai.runtime.failed.is_cooling_down(83, 55.0));
        assert_eq!(p.craft_ai.runtime.item.count_done, 1);
        assert_eq!(p.craft_ai.item_to_craft_name.as_deref(), Some("Fire"));
        // Birth wipe
        p.wipe_craft_on_birth();
        assert_eq!(p.craft_ai.item_to_craft_id, -1);
        assert!(p.craft_ai.crafting_tasks.is_empty());
        assert!(p.craft_ai.runtime.failed.last_fail_sec.is_empty());
        assert!(p.craft_ai.item_to_craft_name.is_none());
        p.craft_ai.runtime.called_craft_item = true;
        p.craft_ai_begin_tick();
        assert!(!p.craft_ai.runtime.called_craft_item);
        // MAKE order → sticky continue before count init
        let say = p
            .craft_ai
            .do_make_craft_command(152, Some("Bow".into()), false);
        assert_eq!(say.as_deref(), Some("Making Bow"));
        assert!(p.craft_ai.should_continue_sticky_craft());
    }

    #[test]
    fn soul_sticky_on_player_defaults_and_survives() {
        // AI-SOUL-WIRE: Player.soul sticky (Haxe playerSoul)
        let mut p = Player::new(1, 1, "soul@test");
        assert_eq!(p.soul.memory_len(), 0);
        assert_eq!(p.soul.chat_len(), 0);
        p.soul
            .add_interaction_default(7, "Bob", "Snow", crate::InteractionType::AttackDamage, 2.0);
        assert_eq!(p.soul.memory_len(), 1);
        assert!((p.soul.interaction(7).unwrap().attack_damage - 2.0).abs() < 1e-4);
        p.soul
            .add_interaction_default(7, "Bob", "Snow", crate::InteractionType::GivenCoins, 5.0);
        assert_eq!(p.soul.memory_len(), 1);
        assert!((p.soul.interaction(7).unwrap().given_coins - 5.0).abs() < 1e-4);
        p.soul
            .add_chat_entry_default(7, "Bob", "Snow", "hi", "hello");
        assert_eq!(p.soul.chat_len(), 1);
    }

    #[test]
    fn backpack_store_clears_uses() {
        let mut p = Player::new(1, 1, "bp@test");
        p.set_held(33, 5);
        assert_eq!(p.store_to_backpack(), Ok(33));
        assert_eq!(p.held_id, 0);
        assert_eq!(p.held_uses, 0);
        assert_eq!(p.backpack, vec![33]);
    }

    #[test]
    fn store_take_worn_pack_dual_writes_nest() {
        // BACKPACK-NEST-DUAL: STORE/TAKE with clothing[5] mutates nest + flat.
        let mut p = Player::new(1, 1, "nestbp@test");
        p.set_clothing_index_helper(5, Some(NestedHelper::id_only(198)));
        p.set_held(33, 5);
        assert_eq!(p.store_to_backpack(), Ok(33));
        assert_eq!(p.held_id, 0);
        assert_eq!(p.backpack, vec![33]);
        assert_eq!(p.backpack_cargo_len(), 1);
        assert_eq!(
            p.clothing_helpers[5].as_ref().unwrap().contained[0].id,
            33
        );
        assert_eq!(p.inv_report(), "INV 1/8 33");
        assert_eq!(p.take_from_backpack(0), Ok(33));
        assert_eq!(p.held_id, 33);
        assert!(p.backpack.is_empty());
        assert!(p.clothing_helpers[5]
            .as_ref()
            .unwrap()
            .contained
            .is_empty());
        assert_eq!(p.inv_report(), "INV 0/8");
    }

    #[test]
    fn wear_pack_with_cargo_syncs_flat_backpack() {
        let mut p = Player::new(1, 1, "wearpack@test");
        p.set_clothing_index_helper(5, Some(NestedHelper::from_wire(198, &[10, 20])));
        assert_eq!(p.backpack, vec![10, 20]);
        assert_eq!(p.backpack_cargo_ids(), vec![10, 20]);
        p.set_clothing_index_helper(5, None);
        assert!(p.backpack.is_empty());
    }

    #[test]
    fn death_loot_prefers_nest_cargo_not_double_flat() {
        let mut p = Player::new(1, 1, "diepack@test");
        p.set_clothing_index_helper(5, Some(NestedHelper::from_wire(198, &[33, 34])));
        let loot = p.take_death_loot_for_scatter();
        assert_eq!(loot, vec![33, 34, 198]);
        assert!(p.backpack.is_empty());
        assert!(p.clothing_helpers[5].is_none());
    }

    #[test]
    fn dropall_drains_nest_keeps_worn_pack() {
        let mut p = Player::new(1, 1, "dropallpack@test");
        p.set_clothing_index_helper(5, Some(NestedHelper::from_wire(198, &[33])));
        p.set_held(55, 0);
        let loot = p.take_dropall_for_scatter();
        assert_eq!(loot, vec![55, 33]);
        assert_eq!(p.clothing_helpers[5].as_ref().unwrap().id, 198);
        assert!(p.clothing_helpers[5]
            .as_ref()
            .unwrap()
            .contained
            .is_empty());
        assert!(p.backpack.is_empty());
    }

    #[test]
    fn clothing_slot_hatchet_is_not_a_hat() {
        assert_eq!(clothing_slot_for_object("Stone Hatchet", ""), None);
        assert_eq!(clothing_slot_for_object("Small Hatchet", ""), None);
        assert_eq!(clothing_slot_for_object("Wool Hat", ""), None);
        assert_eq!(
            clothing_slot_for_object("Wool Hat", "Wool Hat\nclothing=h"),
            Some(ClothingSlot::Hat)
        );
        assert_eq!(
            clothing_slot_for_object_ex("Stone Hatchet", "", "n"),
            None
        );
        assert_eq!(
            clothing_slot_for_object_ex("Wool Hat", "", "h"),
            Some(ClothingSlot::Hat)
        );
    }

    #[test]
    fn wear_and_strip_round_trip() {
        let mut p = Player::new(1, 1, "c@t");
        p.set_held(99, 0);
        assert_eq!(p.wear_held(ClothingSlot::Hat).unwrap(), (99, 0));
        assert_eq!(p.hat, 99);
        assert_eq!(p.held_id, 0);
        assert_eq!(p.strip_slot(ClothingSlot::Hat).unwrap(), 99);
        assert_eq!(p.held_id, 99);
        assert_eq!(p.hat, 0);
    }

    #[test]
    fn clothing_parent_ids_and_snapshot_six_slots() {
        // Haxe: clothingObjects[0..5] → PlayerSnapshot.clothing (DROP-HELD-TABLE quiver)
        let mut p = Player::new(1, 1, "cloth@test");
        p.set_clothing(ClothingSlot::Hat, 586);
        // Empty Arrow Quiver 874 in backpack slot (Haxe clothingObjects[5])
        p.set_clothing_index_helper(5, Some(NestedHelper::with_uses(874, 2)));
        let ids = p.clothing_parent_ids();
        let uses = p.clothing_uses_remaining();
        assert_eq!(ids[0], 586);
        assert_eq!(ids[5], 874);
        assert_eq!(uses[5], 2);
        let snap = p.snapshot();
        assert_eq!(snap.clothing, ids);
        assert_eq!(snap.clothing_uses, uses);
    }

    #[test]
    fn player_snapshot_includes_home_and_profession_sticky() {
        // AI-JOB-SMITH-RESID: home.tx/ty + lastProfession for NPC peer count
        // Haxe: countProfession same-home + lastProfession == 'SMITH'
        let mut p = Player::new(1, 1, "smith-snap@test");
        p.x = 50;
        p.y = 60;
        p.home_x = 10;
        p.home_y = 20;
        assert!(crate::assign_smith_from_speech(
            &mut p.smith_profession,
            "SMITH!"
        ));
        assert!(crate::assign_baker_from_speech(
            &mut p.baker_profession,
            "BAKER!"
        ));
        let snap = p.snapshot();
        assert_eq!(snap.home_x, 10);
        assert_eq!(snap.home_y, 20);
        assert_ne!(snap.home_x, snap.x, "home must not be position proxy");
        assert!(snap.is_last_smith);
        assert!(snap.is_last_baker);
        assert!(!snap.is_last_potter);
        assert!(!snap.is_last_shepherd);
        assert!(!snap.is_last_farm);
        assert!(snap.last_farm.is_none());
        assert!(!snap.is_last_fire_food);
    }

    #[test]
    fn snapshot_last_farm_job_key() {
        // AI-JOB-LIVE-IO-RESID: npc peer-cap uses snapshot last farm when profession_state missing
        let mut p = Player::new(1, 1, "farm-snap@test");
        assert!(crate::assign_farm_from_speech(
            &mut p.farm_profession,
            "WATERBRINGER!"
        ));
        let snap = p.snapshot();
        assert!(snap.is_last_farm);
        assert_eq!(snap.last_farm, Some(crate::FarmProfession::WaterBringer));
    }
}
