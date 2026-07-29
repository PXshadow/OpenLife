//! AiHandler — LLM chat rate limit, prompt assembly, response parse, conversation log.
//!
//! Haxe: `openlife.server.AiHandler` (matrix **AI-HANDLER** / **S-AIH**, chunk `llm_prompt`).
//!
//! Pure helpers only in this module. HTTP provider is **AI-PROVIDER**; speech wire is
//! **AI-LLM-WIRE**. Secrets must come from process env (never `server.toml` / source).
//! No multi-server twin work here.

use crate::player_soul::get_combat_prestige_label;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

// ---------------------------------------------------------------------------
// Defaults (Haxe ServerSettings)
// ---------------------------------------------------------------------------

/// Haxe `ServerSettings.AiCallsPerHour`.
pub const AI_CALLS_PER_HOUR_DEFAULT: u32 = 500;
/// Haxe `ServerSettings.MaxAIResponseperSay`.
pub const MAX_AI_RESPONSE_PER_SAY_DEFAULT: usize = 120;
/// Haxe `ServerSettings.AIWaitTimePer100Chars` (seconds per 100 chars).
pub const AI_WAIT_TIME_PER_100_CHARS_DEFAULT: f32 = 6.0;
/// Sliding window for call timestamps (1 hour).
pub const AI_RATE_WINDOW_SECS: f64 = 3600.0;
/// Haxe `ChatResponse` maxAttempts = initial + 1 retry.
pub const AI_CHAT_MAX_ATTEMPTS: u32 = 2;
/// Haxe default `logFileBaseName`.
pub const AI_CONVERSATION_LOG_BASE_DEFAULT: &str = "log/ai_conversation_log";
/// Haxe splitResponse maxSplits.
pub const AI_RESPONSE_MAX_SPLITS: usize = 10;
/// Separators used when chunking long AI SAY text.
pub const AI_RESPONSE_SEPARATORS: &[&str] = &[".", "!", "?", "*"];

// ---------------------------------------------------------------------------
// Rate limit (Haxe callTimestamps + mutex logic, pure)
// ---------------------------------------------------------------------------

/// Sliding-window call counter for `AiCallsPerHour`.
///
/// // Haxe: AiHandler.callTimestamps / checkRateLimit / recordCall
#[derive(Debug, Clone, Default)]
pub struct AiCallRateLimit {
    /// Unix or monotonic seconds of each recorded call (caller chooses clock).
    timestamps: Vec<f64>,
}

impl AiCallRateLimit {
    pub fn new() -> Self {
        Self::default()
    }

    /// Haxe `cleanOldTimestamps`.
    // Haxe: AiHandler.cleanOldTimestamps
    pub fn clean_old_timestamps(&mut self, now: f64) {
        let cutoff = now - AI_RATE_WINDOW_SECS;
        self.timestamps.retain(|&t| t > cutoff);
    }

    /// Haxe `innerCheckRateLimit` — true if a new call is allowed.
    // Haxe: AiHandler.innerCheckRateLimit
    pub fn check_rate_limit(&mut self, now: f64, limit: u32) -> bool {
        self.clean_old_timestamps(now);
        (self.timestamps.len() as u32) < limit
    }

    /// Haxe `recordCall` — push `now` (caller already passed rate check).
    // Haxe: AiHandler.recordCall
    pub fn record_call(&mut self, now: f64) {
        self.timestamps.push(now);
    }

    /// Haxe `getCurrentCallCount` (cleans first).
    // Haxe: AiHandler.getCurrentCallCount
    pub fn current_call_count(&mut self, now: f64) -> usize {
        self.clean_old_timestamps(now);
        self.timestamps.len()
    }

    /// Test/debug: raw length without clean.
    pub fn raw_len(&self) -> usize {
        self.timestamps.len()
    }
}

/// Haxe `getRateLimit` surface (settings knob).
// Haxe: AiHandler.getRateLimit
#[inline]
pub fn get_rate_limit(ai_calls_per_hour: u32) -> u32 {
    ai_calls_per_hour
}

// ---------------------------------------------------------------------------
// Retry classification (Haxe isNetworkError)
// ---------------------------------------------------------------------------

/// Haxe `AiHandler.isNetworkError` — true → retry once; false → fail immediately.
// Haxe: AiHandler.isNetworkError
pub fn is_network_error(error_msg: &str) -> bool {
    let lower = error_msg.to_lowercase();

    // Network-related errors that might succeed on retry
    const NETWORK: &[&str] = &[
        "connection",
        "timeout",
        "network",
        "socket",
        "dns",
        "refused",
        "reset",
        "unreachable",
        "http error",
    ];
    for p in NETWORK {
        if lower.contains(p) {
            return true;
        }
    }

    // API errors that won't succeed on retry
    const API: &[&str] = &[
        "api key",
        "authentication",
        "unauthorized",
        "forbidden",
        "bad request",
        "invalid",
        "rate limit",
        "quota",
        "payment",
    ];
    for p in API {
        if lower.contains(p) {
            return false;
        }
    }

    // Default: retry unknown errors
    true
}

// ---------------------------------------------------------------------------
// ChatResponse orchestration (provider-injected; no HTTP here)
// ---------------------------------------------------------------------------

/// Result of Haxe `ChatResponse` attempt loop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatResponseOutcome {
    pub response: Option<String>,
    pub attempts: u32,
    pub rate_limited: bool,
}

/// Haxe `ChatResponse` pure control flow: rate limit → record → try/retry on network errors.
///
/// `call_ai` is the inject for **AI-PROVIDER** (`AIProvider.callAi`). Returns `Err` message
/// for classification; `Ok(text)` is success.
// Haxe: AiHandler.ChatResponse
pub fn chat_response_with<F>(
    rate: &mut AiCallRateLimit,
    now: f64,
    limit: u32,
    mut call_ai: F,
) -> ChatResponseOutcome
where
    F: FnMut() -> Result<String, String>,
{
    if !rate.check_rate_limit(now, limit) {
        return ChatResponseOutcome {
            response: None,
            attempts: 0,
            rate_limited: true,
        };
    }
    rate.record_call(now);

    let mut response: Option<String> = None;
    let mut attempts = 0u32;
    let max_attempts = AI_CHAT_MAX_ATTEMPTS;

    while attempts < max_attempts {
        attempts += 1;
        match call_ai() {
            Ok(text) => {
                response = Some(text);
                break;
            }
            Err(msg) => {
                let net = is_network_error(&msg);
                if !net || attempts >= max_attempts {
                    break;
                }
            }
        }
    }

    ChatResponseOutcome {
        response,
        attempts,
        rate_limited: false,
    }
}

// ---------------------------------------------------------------------------
// Relationship + command context (Haxe getRelationshipInfo / checkIfShouldDoCommand)
// ---------------------------------------------------------------------------

/// Precomputed relationship facts for prompt text (main-thread snapshotted).
#[derive(Debug, Clone, Default)]
pub struct RelationshipView {
    pub is_ally: bool,
    pub is_friendly: bool,
    /// toPlayer is exiled by fromPlayer (AI exiled human).
    pub to_exiled_by_from: bool,
    /// fromPlayer is exiled by toPlayer.
    pub from_exiled_by_to: bool,
    /// toPlayer exiled by any leader from fromPlayer's tribe.
    pub to_exiled_by_any_leader_from: bool,
    /// toPlayer has at least one direct follower (not self).
    pub to_has_followers: bool,
    /// toPlayer follows someone or top leader ≠ toPlayer (Haxe gate for follower scan).
    pub to_follows_or_not_top: bool,
    /// toPlayer.followPlayer == fromPlayer.
    pub to_follows_from: bool,
    /// toPlayer top leader == fromPlayer.
    pub from_is_top_of_to: bool,
    /// fromPlayer.followPlayer == toPlayer.
    pub from_follows_to: bool,
    /// fromPlayer top leader == toPlayer.
    pub to_is_top_of_from: bool,
    pub from_is_cursed: bool,
    pub to_is_cursed: bool,
    pub from_lost_combat_prestige: f32,
    pub to_lost_combat_prestige: f32,
}

/// Haxe `AiHandler.getRelationshipInfo`.
// Haxe: AiHandler.getRelationshipInfo
pub fn get_relationship_info(v: &RelationshipView) -> String {
    let mut text = String::new();

    if v.is_ally {
        text.push_str("You are allied with this player (same tribe/family). ");
    }
    if v.is_friendly {
        text.push_str("You are on friendly terms with this player. ");
    }
    if v.to_exiled_by_from {
        text.push_str("You have exiled this player! They are not welcome in your tribe. ");
    }
    if v.from_exiled_by_to {
        text.push_str("This player has exiled you! You are not welcome in their tribe. ");
    }
    if v.to_exiled_by_any_leader_from {
        text.push_str("This player has been exiled by a leader in your tribe. ");
    }

    // Haxe: only mention "leader with followers" when toPlayer follows someone / not top
    if v.to_follows_or_not_top && v.to_has_followers {
        text.push_str("This player is a leader with followers in their tribe. ");
    }

    if v.to_follows_from {
        text.push_str("This player follows you as their leader! ");
    }
    if v.from_is_top_of_to {
        text.push_str("You are the top leader of this player's tribe! ");
    }
    if v.from_follows_to {
        text.push_str("You follow this player as your leader! ");
    }
    if v.to_is_top_of_from {
        text.push_str("This player is the top leader of your tribe! ");
    }

    if v.from_is_cursed {
        text.push_str("You are cursed! Others might not trust you!");
    }
    if v.to_is_cursed {
        text.push_str("This player is cursed! Be careful to trust!");
    }

    if v.from_lost_combat_prestige != 0.0 {
        let label = get_combat_prestige_label(v.from_lost_combat_prestige);
        text.push_str(&format!("Your combat reputation is {label}. "));
    }
    if v.to_lost_combat_prestige != 0.0 {
        let label = get_combat_prestige_label(v.to_lost_combat_prestige);
        text.push_str(&format!("This player's combat reputation is {label}. "));
    }
    if v.to_lost_combat_prestige > 1.0 {
        text.push_str(
            "Be very careful around players with bad combat reputation especially if not allied!",
        );
    }

    text
}

/// Haxe `AiHandler.checkIfShouldDoCommand`.
// Haxe: AiHandler.checkIfShouldDoCommand
pub fn check_if_should_do_command(from_is_follower_of_to: bool, is_close_relative: bool) -> String {
    if from_is_follower_of_to {
        return "You are a follower of this player! Therefore if asked you should do commands!"
            .to_string();
    }
    // Haxe TODO: check if exiled
    if is_close_relative {
        return "You are a close relative of this player so if asked you should help him / her!"
            .to_string();
    }
    "You are not a follower of this player, so if asked you can reject commands of this player!"
        .to_string()
}

/// Haxe `AiHandler.getCommandContext` — JSON action schema for the model.
// Haxe: AiHandler.getCommandContext
pub fn get_command_context() -> &'static str {
    "Always respond with valid JSON including a fitting emote and any actions you want to perform:
{ \"text\": \"your response\", \"emote\": \"emoteName\" }
Add only the actions you want to perform:
- \"text\": your roleplay response
- \"emote\": one ONLY: happy, angry, love, sad, joy, blush, devious, shock, terrified, homesick, mad, oreally, ill, hmph, snowSplat
- \"followPlayer\": true - to stay close / walk with the player
- \"drop\": true - to drop held item
- \"makeItem\": \"item name or id\" - to craft an item (e.g. \"knife\" or 71)

Examples:
{ \"text\": \"I will follow you!\", \"emote\": \"happy\", \"followPlayer\": true }
{ \"text\": \"Making a knife.\", \"emote\": \"joy\", \"makeItem\": \"knife\" }
{ \"text\": \"Hello!\", \"emote\": \"joy\" }"
}

// ---------------------------------------------------------------------------
// buildPrompt
// ---------------------------------------------------------------------------

/// Inputs already built from PlayerSoul (main thread).
#[derive(Debug, Clone)]
pub struct PromptParts {
    pub own_context: String,
    pub other_context: String,
    pub relationship_context: String,
    pub do_command_text: String,
    pub memory_context: String,
    pub chat_memory_context: String,
    pub message: String,
}

/// Haxe `AiHandler.buildPrompt`.
// Haxe: AiHandler.buildPrompt
pub fn build_prompt(parts: &PromptParts) -> String {
    let mut prompt = format!(
        "{}\n{}\n{}\n{}",
        parts.own_context,
        parts.other_context,
        parts.relationship_context,
        parts.do_command_text
    );

    if !parts.memory_context.is_empty() {
        prompt.push('\n');
        prompt.push_str(&parts.memory_context);
    }
    if !parts.chat_memory_context.is_empty() {
        prompt.push('\n');
        prompt.push_str(&parts.chat_memory_context);
    }

    prompt.push('\n');
    prompt.push_str(get_command_context());
    prompt.push('\n');
    prompt.push_str(
        "The other player says to you respond in your role considering your status / prestige and the other players status / prestige: ",
    );
    prompt.push_str(&parts.message);
    prompt
}

// ---------------------------------------------------------------------------
// Emote map + parseAiResponse (pure side-effects as structured actions)
// ---------------------------------------------------------------------------

/// Haxe `AiHandler.getEmoteId` — `-1` if unknown.
// Haxe: AiHandler.getEmoteId
pub fn get_emote_id(emote_name: &str) -> i32 {
    match emote_name.to_lowercase().as_str() {
        "happy" => 0,
        "mad" => 1,
        "angry" => 2,
        "sad" => 3,
        "devious" => 4,
        "joy" => 5,
        "blush" => 6,
        "snowsplat" => 8,
        "oreally" => 14,
        "ill" => 10,
        "hmph" => 12,
        "shock" => 15,
        "love" => 13,
        "terrified" => 27,
        "homesick" => 28,
        _ => -1,
    }
}

/// Actions extracted from AI JSON (applied by live wire later).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AiResponseActions {
    pub follow_player: bool,
    pub drop: bool,
    pub make_item: Option<String>,
}

/// Pure result of Haxe `parseAiResponse` (without mutating player).
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedAiResponse {
    /// SAY text (JSON `text` or raw response).
    pub text: String,
    /// Emote id if valid name present; `-1` = none.
    pub emote_id: i32,
    pub actions: AiResponseActions,
    /// True when JSON parse succeeded (even if sparse).
    pub was_json: bool,
}

/// Haxe `AiHandler.parseAiResponse` pure extraction.
// Haxe: AiHandler.parseAiResponse
pub fn parse_ai_response(response: &str) -> ParsedAiResponse {
    let mut out = ParsedAiResponse {
        text: response.to_string(),
        emote_id: -1,
        actions: AiResponseActions::default(),
        was_json: false,
    };

    let Ok(json) = serde_json::from_str::<Value>(response) else {
        return out;
    };
    out.was_json = true;

    if let Some(t) = json.get("text").and_then(|v| v.as_str()) {
        out.text = t.to_string();
    }

    if let Some(emote_name) = json.get("emote").and_then(|v| v.as_str()) {
        let id = get_emote_id(emote_name);
        if id >= 0 {
            out.emote_id = id;
        }
    }

    if json
        .get("followPlayer")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        out.actions.follow_player = true;
    }
    if json.get("drop").and_then(|v| v.as_bool()).unwrap_or(false) {
        out.actions.drop = true;
    }
    if let Some(item) = json.get("makeItem") {
        if !item.is_null() {
            let s = match item {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                other => other.to_string(),
            };
            if !s.is_empty() && s != "null" {
                out.actions.make_item = Some(s);
            }
        }
    }

    out
}

// ---------------------------------------------------------------------------
// Conversation log (format only; I/O in ol-server or caller)
// ---------------------------------------------------------------------------

/// Haxe `getDateString` — `YYYY-MM-DD` from civil date parts (month 1–12).
// Haxe: AiHandler.getDateString
pub fn format_date_string(year: i32, month: u32, day: u32) -> String {
    format!("{year:04}-{month:02}-{day:02}")
}

/// Haxe `getCurrentLogFilePath`.
// Haxe: AiHandler.getCurrentLogFilePath
pub fn conversation_log_path(base_name: &str, date_yyyy_mm_dd: &str) -> PathBuf {
    PathBuf::from(format!("{base_name}_{date_yyyy_mm_dd}.txt"))
}

/// Haxe `logToFile` entry body (without directory create / append I/O).
// Haxe: AiHandler.logToFile
pub fn format_conversation_log_entry(
    timestamp: &str,
    log_file_path: &str,
    full_prompt: &str,
    response: Option<&str>,
) -> String {
    let response_body = response.unwrap_or("(null - failed or rate limited)");
    format!(
        "========================================\n\
         Timestamp: {timestamp}\n\
         Log File: {log_file_path}\n\n\
         --- FULL PROMPT ---\n\
         {full_prompt}\n\n\
         --- RESPONSE ---\n\
         {response_body}\n\
         ========================================\n\n"
    )
}

/// Ensure parent `log` directory exists (Haxe `sys.FileSystem.createDirectory("log")`).
// Haxe: AiHandler.logToFile ensure log dir
pub fn ensure_log_dir(log_dir: &Path) -> std::io::Result<()> {
    if !log_dir.exists() {
        std::fs::create_dir_all(log_dir)?;
    }
    Ok(())
}

/// Append a conversation log entry to `path` (creates parent dirs from path parent).
pub fn append_conversation_log(path: &Path, entry: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        ensure_log_dir(parent)?;
    }
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    f.write_all(entry.as_bytes())?;
    Ok(())
}

/// UTC civil Y-M-D from days since Unix epoch (1970-01-01).
// Haxe: Date local civil; Rust logs use UTC for portability
fn civil_ymd_from_unix_days(days: i64) -> (i32, u32, u32) {
    // Howard Hinnant civil_from_days
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m as u32, d as u32)
}

/// `(YYYY-MM-DD, "YYYY-MM-DD HH:MM:SS")` from Unix epoch seconds (UTC).
// Haxe: getDateString + Date.now().toString
pub fn format_log_timestamp_from_unix(secs: u64) -> (String, String) {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let hh = (rem / 3600) as u32;
    let mm = ((rem % 3600) / 60) as u32;
    let ss = (rem % 60) as u32;
    let (y, m, d) = civil_ymd_from_unix_days(days);
    let date = format_date_string(y, m, d);
    let ts = format!("{date} {hh:02}:{mm:02}:{ss:02}");
    (date, ts)
}

/// Haxe `logToFile` — format entry and append under daily path (`log_base_YYYY-MM-DD.txt`).
///
/// Best-effort; callers may ignore I/O errors (Haxe traces and continues).
// Haxe: AiHandler.logToFile
pub fn log_conversation_to_file(
    log_base: &str,
    date_yyyy_mm_dd: &str,
    timestamp: &str,
    full_prompt: &str,
    response: Option<&str>,
) -> std::io::Result<PathBuf> {
    let path = conversation_log_path(log_base, date_yyyy_mm_dd);
    let path_display = path.to_string_lossy().into_owned();
    let entry =
        format_conversation_log_entry(timestamp, &path_display, full_prompt, response);
    append_conversation_log(&path, &entry)?;
    Ok(path)
}

/// Wall-clock `logToFile` using Unix-UTC date/timestamp (drain worker convenience).
// Haxe: AiHandler.logToFile on respondToPlayerAsync Thread
pub fn log_conversation_to_file_now(
    log_base: &str,
    full_prompt: &str,
    response: Option<&str>,
) -> std::io::Result<PathBuf> {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (date, ts) = format_log_timestamp_from_unix(secs);
    log_conversation_to_file(log_base, &date, &ts, full_prompt, response)
}

// ---------------------------------------------------------------------------
// respondToPlayerAsync pure stages
// ---------------------------------------------------------------------------

/// Main-thread snapshot for async LLM worker (Haxe builds prompt on main thread).
#[derive(Debug, Clone)]
pub struct RespondToPlayerPlan {
    pub full_prompt: String,
    /// AI player id (for live apply of emote/actions).
    pub from_player_id: i32,
    /// Human speaker id (chat memory).
    pub to_player_id: i32,
    pub to_player_name: String,
    pub to_player_family_name: String,
    pub human_message: String,
}

/// Build async plan: prompt assembly only (Haxe main-thread half of `respondToPlayerAsync`).
// Haxe: AiHandler.respondToPlayerAsync (prompt build)
pub fn plan_respond_to_player(
    parts: &PromptParts,
    from_player_id: i32,
    to_player_id: i32,
    to_player_name: &str,
    to_player_family_name: &str,
) -> RespondToPlayerPlan {
    RespondToPlayerPlan {
        full_prompt: build_prompt(parts),
        from_player_id,
        to_player_id,
        to_player_name: to_player_name.to_string(),
        to_player_family_name: to_player_family_name.to_string(),
        human_message: parts.message.clone(),
    }
}

/// Collapse newlines like Haxe `response.split("\n").join(" ")`.
// Haxe: respondToPlayerAsync response newline collapse
pub fn collapse_response_newlines(response: &str) -> String {
    response.split('\n').collect::<Vec<_>>().join(" ")
}

/// After LLM returns: parse + chunk plan for SAY fan-out.
#[derive(Debug, Clone, PartialEq)]
pub struct RespondProcessResult {
    pub parsed: ParsedAiResponse,
    /// Chunks to pass to onSuccess (Haxe sendResponseInChunks).
    pub say_chunks: Vec<String>,
    /// Wait seconds per chunk (Haxe AIWaitTimePer100Chars).
    pub wait_secs_per_chunk: Vec<f32>,
    /// Whether chat memory should record (response was non-null after call).
    pub record_chat_memory: bool,
}

/// Haxe post-LLM half of `respondToPlayerAsync` + `sendResponseInChunks` (pure).
// Haxe: AiHandler.respondToPlayerAsync / sendResponseInChunks
pub fn process_llm_response_for_say(
    raw_response: Option<&str>,
    max_len: usize,
    wait_per_100: f32,
) -> RespondProcessResult {
    let Some(raw) = raw_response else {
        return RespondProcessResult {
            parsed: ParsedAiResponse {
                text: String::new(),
                emote_id: -1,
                actions: AiResponseActions::default(),
                was_json: false,
            },
            say_chunks: vec![],
            wait_secs_per_chunk: vec![],
            record_chat_memory: false,
        };
    };

    let collapsed = collapse_response_newlines(raw);
    let parsed = parse_ai_response(&collapsed);
    let (chunks, waits) = plan_response_chunks(&parsed.text, max_len, wait_per_100);

    RespondProcessResult {
        parsed,
        say_chunks: chunks,
        wait_secs_per_chunk: waits,
        record_chat_memory: true,
    }
}

/// Haxe `sendResponseInChunks` pure split + wait times.
// Haxe: AiHandler.sendResponseInChunks
pub fn plan_response_chunks(
    response: &str,
    max_len: usize,
    wait_per_100: f32,
) -> (Vec<String>, Vec<f32>) {
    let needs_split = response.len() > max_len && contains_any_separator(response, AI_RESPONSE_SEPARATORS);
    if !needs_split {
        let wait = wait_time_for_chars(response.len(), wait_per_100);
        return (vec![response.to_string()], vec![wait]);
    }
    let chunks = split_response(
        response,
        max_len,
        AI_RESPONSE_SEPARATORS,
        AI_RESPONSE_MAX_SPLITS,
    );
    let waits: Vec<f32> = chunks
        .iter()
        .map(|c| wait_time_for_chars(c.len(), wait_per_100))
        .collect();
    (chunks, waits)
}

// Haxe: AiHandler containsAnySeparator
pub fn contains_any_separator(text: &str, separators: &[&str]) -> bool {
    separators.iter().any(|sep| text.contains(sep))
}

/// Haxe wait: `(len / 100) * AIWaitTimePer100Chars`.
// Haxe: AiHandler.sendResponseInChunks waitTime
#[inline]
pub fn wait_time_for_chars(len: usize, wait_per_100: f32) -> f32 {
    (len as f32 / 100.0) * wait_per_100
}

/// Haxe `AiHandler.splitResponse`.
// Haxe: AiHandler.splitResponse
pub fn split_response(
    text: &str,
    max_len: usize,
    separators: &[&str],
    max_splits: usize,
) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut remaining = text.to_string();

    while !remaining.is_empty() && chunks.len() < max_splits {
        if remaining.len() <= max_len {
            chunks.push(remaining);
            break;
        }

        // Find best separator at or before maxLen (lastIndexOf)
        let mut split_index: isize = -1;
        for sep in separators {
            if let Some(idx) = last_index_of_before(&remaining, sep, max_len) {
                if idx as isize > split_index {
                    split_index = idx as isize;
                }
            }
        }

        // If none, first separator after maxLen
        if split_index == -1 {
            let mut earliest_after = remaining.len();
            for sep in separators {
                if let Some(idx) = remaining[max_len.min(remaining.len())..].find(sep) {
                    let abs = max_len.min(remaining.len()) + idx;
                    if abs < earliest_after {
                        earliest_after = abs;
                    }
                }
            }
            if earliest_after == remaining.len() {
                split_index = -1;
            } else {
                split_index = earliest_after as isize;
            }
        }

        if split_index > 0 {
            let end = (split_index as usize) + 1;
            let end = end.min(remaining.len());
            chunks.push(remaining[..end].to_string());
            remaining = remaining[end..].to_string();
        } else {
            let take = max_len.min(remaining.len());
            chunks.push(remaining[..take].to_string());
            remaining = remaining[take..].to_string();
        }
    }

    chunks
}

/// Haxe `lastIndexOf(sep, maxLen)` — last occurrence of `sep` starting at index ≤ max_len.
fn last_index_of_before(s: &str, sep: &str, max_len: usize) -> Option<usize> {
    if sep.is_empty() || s.is_empty() {
        return None;
    }
    // Highest index i where i <= max_len and s[i..] starts with sep (Haxe lastIndexOf fromIndex).
    let mut best: Option<usize> = None;
    let mut start = 0usize;
    while start <= max_len && start < s.len() {
        if let Some(rel) = s[start..].find(sep) {
            let abs = start + rel;
            if abs <= max_len {
                best = Some(abs);
                start = abs + 1;
            } else {
                break;
            }
        } else {
            break;
        }
    }
    best
}

// ---------------------------------------------------------------------------
// Env secrets helpers (no keys in source; used by ol-server)
// ---------------------------------------------------------------------------

/// Sentinel Haxe uses when key unset: `"Not Set"`.
pub const AI_API_KEY_NOT_SET: &str = "Not Set";

/// Haxe `AIProvider.IsLLMActivated` — key present and not the Not Set sentinel.
// Haxe: AIProvider.IsLLMActivated
pub fn is_llm_activated(api_key: Option<&str>) -> bool {
    match api_key {
        Some(k) if !k.is_empty() && k != AI_API_KEY_NOT_SET => true,
        _ => false,
    }
}

/// Load API key from env only. Prefers `AI_API_KEY`, then `XAI_API_KEY`.
/// Never reads from `server.toml` / SecretOmit fields.
pub fn api_key_from_env() -> Option<String> {
    std::env::var("AI_API_KEY")
        .ok()
        .filter(|s| !s.is_empty() && s != AI_API_KEY_NOT_SET)
        .or_else(|| {
            std::env::var("XAI_API_KEY")
                .ok()
                .filter(|s| !s.is_empty() && s != AI_API_KEY_NOT_SET)
        })
}

/// Base URL from env `AI_API_URL` (Haxe default is MiniMax anthropic path — provider chunk).
pub fn api_url_from_env() -> Option<String> {
    std::env::var("AI_API_URL")
        .ok()
        .filter(|s| !s.is_empty())
}

/// Model from env `AI_DEFAULT_MODEL`.
pub fn default_model_from_env() -> Option<String> {
    std::env::var("AI_DEFAULT_MODEL")
        .ok()
        .filter(|s| !s.is_empty())
}

/// Max tokens from env (`AI_MAX_TOKENS` or `AI_MAX_TOKENS_FOR_CHAT`).
// Haxe: ServerSettings.AiMaxTokensForChat
pub fn max_tokens_for_chat_from_env() -> Option<u32> {
    std::env::var("AI_MAX_TOKENS")
        .ok()
        .and_then(|s| s.parse().ok())
        .filter(|&n| n > 0)
        .or_else(|| {
            std::env::var("AI_MAX_TOKENS_FOR_CHAT")
                .ok()
                .and_then(|s| s.parse().ok())
                .filter(|&n| n > 0)
        })
}

// ---------------------------------------------------------------------------
// AI-PROVIDER pure (request body + parseResponse; no HTTP I/O)
// Haxe: openlife.server.AIProvider — also re-exported via `ai_provider` module
// ---------------------------------------------------------------------------

/// Haxe `ServerSettings.AiApiUrl` default.
pub const AI_DEFAULT_API_URL: &str = "https://api.minimax.io/anthropic";
/// Haxe `ServerSettings.AiDefaultModel`.
pub const AI_DEFAULT_MODEL: &str = "MiniMax-M2.5-highspeed";
/// Haxe `ServerSettings.AiMaxTokensForChat`.
pub const AI_MAX_TOKENS_FOR_CHAT_DEFAULT: u32 = 1024;
/// Haxe system role content in `AIProvider.callAi`.
pub const AI_SYSTEM_DIALOG_PROMPT: &str = "This is a interactiv dialog! No thinking needed! Respond fast with one or two sentences and stay in your role!";
/// Haxe `anthropic-version` header.
pub const AI_ANTHROPIC_VERSION: &str = "2023-06-01";

/// Resolve base API URL: env override or Haxe default.
// Haxe: ServerSettings.AiApiUrl
pub fn resolve_ai_api_url(env_url: Option<&str>) -> String {
    env_url
        .filter(|s| !s.is_empty())
        .unwrap_or(AI_DEFAULT_API_URL)
        .to_string()
}

/// Resolve model: per-call override → env → Haxe default.
// Haxe: AIProvider.callAi useModel
pub fn resolve_ai_model(override_model: Option<&str>, env_model: Option<&str>) -> String {
    if let Some(m) = override_model.filter(|s| !s.is_empty()) {
        return m.to_string();
    }
    if let Some(m) = env_model.filter(|s| !s.is_empty()) {
        return m.to_string();
    }
    AI_DEFAULT_MODEL.to_string()
}

/// Haxe `` `${AiApiUrl}/v1/messages` ``.
// Haxe: AIProvider.callAi Http path
pub fn ai_messages_endpoint(api_url_base: &str) -> String {
    let base = api_url_base.trim_end_matches('/');
    format!("{base}/v1/messages")
}

/// HTTP headers for MiniMax/Anthropic-compatible POST.
// Haxe: AIProvider.callAi setHeader
pub fn ai_request_headers(api_key: &str) -> Vec<(String, String)> {
    vec![
        ("Content-Type".into(), "application/json".into()),
        ("Authorization".into(), format!("Bearer {api_key}")),
        ("x-api-key".into(), api_key.to_string()),
        ("anthropic-version".into(), AI_ANTHROPIC_VERSION.into()),
    ]
}

/// Build POST JSON body for `AIProvider.callAi`.
// Haxe: AIProvider.callAi requestBody
pub fn build_ai_request_body(prompt: &str, model: &str, max_tokens: u32) -> String {
    let body = serde_json::json!({
        "model": model,
        "max_tokens": max_tokens,
        "messages": [
            {
                "role": "system",
                "content": AI_SYSTEM_DIALOG_PROMPT
            },
            {
                "role": "user",
                "content": prompt
            }
        ]
    });
    body.to_string()
}

/// Guard: refuse call when key missing (Haxe throws before HTTP).
// Haxe: AIProvider.callAi AiApiKey == "Not Set"
pub fn ensure_api_key_configured(api_key: Option<&str>) -> Result<&str, String> {
    match api_key {
        Some(k) if is_llm_activated(Some(k)) => Ok(k),
        _ => Err("AI API key not configured. Set AI_API_KEY or XAI_API_KEY env".into()),
    }
}

/// Haxe `AIProvider.parseResponse` — extract assistant text from provider JSON.
// Haxe: AIProvider.parseResponse
pub fn parse_provider_response(response_json: &str) -> Result<String, String> {
    if response_json.trim().is_empty() {
        return Err("AI empty response".into());
    }
    let response: Value = serde_json::from_str(response_json)
        .map_err(|e| format!("Failed to parse AI response: {e}"))?;

    if response.get("type").and_then(|v| v.as_str()) == Some("error") {
        let msg = response
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("AI API error");
        return Err(msg.to_string());
    }

    // MiniMax / Anthropic: content: [{ type: "text", text: "..." }, ...]
    if let Some(content) = response.get("content").and_then(|v| v.as_array()) {
        let mut collected_text = String::new();
        for block in content {
            if block.is_null() {
                continue;
            }
            let ty = block.get("type").and_then(|v| v.as_str()).unwrap_or("");
            if ty == "text" {
                if let Some(t) = block.get("text").and_then(|v| v.as_str()) {
                    collected_text.push_str(t);
                }
            } else if ty != "thinking" {
                if let Some(t) = block.get("text").and_then(|v| v.as_str()) {
                    collected_text.push_str(t);
                }
            }
        }
        let trimmed = collected_text.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }

    // OpenAI-style: choices[0].message.content
    if let Some(choices) = response.get("choices").and_then(|v| v.as_array()) {
        if let Some(first) = choices.first() {
            if let Some(content) = first
                .get("message")
                .and_then(|m| m.get("content"))
                .and_then(|c| c.as_str())
            {
                return Ok(content.to_string());
            }
        }
    }

    Err("AI response format not recognized".into())
}

// ---------------------------------------------------------------------------
// Apply plan from parseAiResponse (pure plan; live → ai_llm_apply / AI-LLM-APPLY)
// ---------------------------------------------------------------------------

/// Haxe `doEmote(emoteId, 300)` second arg.
pub const AI_EMOTE_SECONDS: i32 = 300;

/// Pure side-effect plan after `parse_ai_response` (no player mutation here).
// Haxe: AiHandler.parseAiResponse doEmote / startFollowingPlayer / doDrop / doMakeCraft
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ApplyAiResponsePlan {
    /// Emote id when ≥ 0 from parse.
    pub emote_id: Option<i32>,
    /// Always 300 when emote present (Haxe).
    pub emote_seconds: i32,
    pub follow_player: bool,
    pub drop: bool,
    pub make_item: Option<String>,
    /// Haxe `doMakeCraftCommand(name, true)`.
    pub make_item_force: bool,
}

/// Build apply plan from parsed AI JSON (pure).
// Haxe: AiHandler.parseAiResponse side effects
pub fn plan_apply_parsed_ai_response(parsed: &ParsedAiResponse) -> ApplyAiResponsePlan {
    let mut plan = ApplyAiResponsePlan {
        emote_id: None,
        emote_seconds: AI_EMOTE_SECONDS,
        follow_player: parsed.actions.follow_player,
        drop: parsed.actions.drop,
        make_item: parsed.actions.make_item.clone(),
        make_item_force: true,
    };
    if parsed.emote_id >= 0 {
        plan.emote_id = Some(parsed.emote_id);
    }
    if plan.make_item.is_none() {
        plan.make_item_force = false;
    }
    plan
}

// ---------------------------------------------------------------------------
// AI-LLM-WIRE speech gate (pure; live tick wire residual)
// Haxe: AiBase speech fallback ~4971
// ---------------------------------------------------------------------------

/// Haxe `myPlayer.age > 3`.
pub const LLM_SPEECH_MIN_AGE: f32 = 3.0;
/// Haxe `timePassedInSeconds > 4`.
pub const LLM_SPEECH_COOLDOWN_SECS: f32 = 4.0;
/// Haxe `Emote.oreally` while thinking.
pub const LLM_SPEECH_ACK_EMOTE_ID: i32 = 14;
/// Haxe `setWaitingTimeMin(6)` when ally stop.
pub const LLM_SPEECH_ALLY_WAIT_SECS: f32 = 6.0;
/// Haxe interim `myPlayer.say("...")`.
pub const LLM_SPEECH_THINKING_SAY: &str = "...";

/// Snapshot for Haxe speech→LLM gate.
#[derive(Debug, Clone)]
pub struct SpeechLlmGate {
    pub speaker_is_human: bool,
    pub llm_activated: bool,
    pub ai_age: f32,
    pub text: String,
    /// Seconds since last LLM react (`CalculateTimeSinceTicksInSec`).
    pub time_since_last_react_secs: f32,
    /// Haxe `timeReactedLastCommand < 1` (never / reset).
    pub never_reacted: bool,
}

/// Haxe AiBase speech fallback predicate (before emote/say/async).
// Haxe: AiBase ~4974 IsLLMActivated / age / !? / cooldown
pub fn should_invoke_llm_for_speech(g: &SpeechLlmGate) -> bool {
    if !g.speaker_is_human {
        return false;
    }
    if !g.llm_activated {
        return false;
    }
    if g.ai_age <= LLM_SPEECH_MIN_AGE {
        return false;
    }
    if g.text.starts_with('!') || g.text.starts_with('?') {
        return false;
    }
    if g.never_reacted {
        return true;
    }
    g.time_since_last_react_secs > LLM_SPEECH_COOLDOWN_SECS
}

/// Immediate UI plan when speech gate passes (before `respondToPlayerAsync`).
// Haxe: AiBase doEmote(oreally) / ally stop / say "..."
#[derive(Debug, Clone, PartialEq)]
pub struct SpeechLlmImmediatePlan {
    pub emote_id: i32,
    pub thinking_say: &'static str,
    /// When true, live wire should stop movement (Goto self) + setWaitingTimeMin.
    pub stop_and_wait_if_ally: bool,
    pub ally_wait_secs: f32,
}

/// Fixed immediate plan for LLM speech ack (ally stop decided by live check).
// Haxe: AiBase speech fallback immediate side effects
pub fn plan_speech_llm_immediate() -> SpeechLlmImmediatePlan {
    SpeechLlmImmediatePlan {
        emote_id: LLM_SPEECH_ACK_EMOTE_ID,
        thinking_say: LLM_SPEECH_THINKING_SAY,
        stop_and_wait_if_ally: true,
        ally_wait_secs: LLM_SPEECH_ALLY_WAIT_SECS,
    }
}

// ---------------------------------------------------------------------------
// AI-LLM-WIRE — hear fan-out, ally speech, runtime, complete plan (pure)
// Haxe: Connection.sendSayToAllClose → AiBase.say / sayHelper / checkIfYouAreAllied
// ---------------------------------------------------------------------------

/// Haxe `ServerSettings.MaxDistanceToBeConsideredAsCloseForSayAi`.
pub const MAX_DISTANCE_SAY_AI: f32 = 20.0;
/// Haxe `Emote.angry` when non-ally loud reject.
pub const LLM_SPEECH_ANGRY_EMOTE_ID: i32 = 2;
/// Haxe `myPlayer.say('I AM NOT YOUR ALLY!')`.
pub const LLM_NOT_ALLY_SAY: &str = "I AM NOT YOUR ALLY!";

/// Squared Euclidean distance (Haxe `AiHelper.CalculateDistanceToPlayer` quad-dist).
// Haxe: AiHelper.CalculateDistanceToPlayer
#[inline]
pub fn ai_say_quad_distance(ax: i32, ay: i32, bx: i32, by: i32) -> f32 {
    let dx = (bx - ax) as f32;
    let dy = (by - ay) as f32;
    dx * dx + dy * dy
}

/// Haxe `quadDist > Math.pow(MaxDistanceToBeConsideredAsCloseForSayAi, 2)` → too far.
// Haxe: AiBase.sayHelper distance gate
#[inline]
pub fn ai_within_say_range(quad_dist: f32, max_dist: f32) -> bool {
    quad_dist <= max_dist * max_dist
}

/// Attention filter: ALL / !! / ?? / name → process (strip leading `ALL `);
/// else only the closest living player to the speaker may hear.
///
/// Returns `Some(normalized_text)` when this AI should run `sayHelper`.
// Haxe: AiBase.sayHelper ALL/!!/??/name / getClosestPlayer
pub fn ai_speech_attention(text: &str, ai_name: &str, is_closest_to_speaker: bool) -> Option<String> {
    let upper = text.to_uppercase();
    let name_u = ai_name.to_uppercase();
    let named = !name_u.is_empty() && upper.contains(name_u.as_str());
    if upper.starts_with("ALL ") || upper.contains("!!") || upper.contains("??") || named {
        // Haxe: text.replace("ALL ", "") — first occurrence only style via strip once.
        let mut out = text.to_string();
        if let Some(rest) = text.strip_prefix("ALL ").or_else(|| text.strip_prefix("all ")) {
            out = rest.to_string();
        } else if let Some(idx) = upper.find("ALL ") {
            // mid-string "ALL " unlikely; Haxe replace removes substring
            let end = idx + 4;
            if end <= text.len() {
                out = format!("{}{}", &text[..idx], &text[end..]);
            }
        }
        return Some(out);
    }
    if is_closest_to_speaker {
        Some(text.to_string())
    } else {
        None
    }
}

/// Haxe `AiBase.checkIfYouAreAllied` speech gate.
// Haxe: AiBase.checkIfYouAreAllied
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlliedSpeechOutcome {
    Allowed,
    DeniedSilent,
    DeniedLoud {
        say: &'static str,
        emote_id: i32,
    },
}

/// `silent=true` → no say/emote (LLM pre-check); `silent=false` → angry reject line.
// Haxe: AiBase.checkIfYouAreAllied
pub fn check_if_you_are_allied_speech(is_friendly: bool, silent: bool) -> AlliedSpeechOutcome {
    if is_friendly {
        return AlliedSpeechOutcome::Allowed;
    }
    if silent {
        return AlliedSpeechOutcome::DeniedSilent;
    }
    AlliedSpeechOutcome::DeniedLoud {
        say: LLM_NOT_ALLY_SAY,
        emote_id: LLM_SPEECH_ANGRY_EMOTE_ID,
    }
}

/// Sticky LLM speech runtime on AI players (Haxe `timeReactedLastCommand` + chunk waits).
// Haxe: AiBase.timeReactedLastCommand / setWaitingTimeMin / sendResponseInChunks
#[derive(Debug, Clone, Default)]
pub struct LlmSpeechRuntime {
    /// Sim time of last LLM reaction; `0.0` = never (Haxe `timeReactedLastCommand < 1`).
    pub last_react_sim_time: f32,
    /// Haxe `waitingTime` / `setWaitingTimeMin` floor.
    pub waiting_time_min: f32,
    /// Async LLM call in flight (prevents re-entry while awaiting).
    pub in_flight: bool,
    /// Chunked SAY queue after LLM returns.
    pub pending_says: std::collections::VecDeque<PendingLlmSayChunk>,
}

/// One delayed SAY chunk from `sendResponseInChunks`.
#[derive(Debug, Clone, PartialEq)]
pub struct PendingLlmSayChunk {
    pub text: String,
    /// Sim time when this chunk may be spoken.
    pub ready_at: f32,
}

/// Build gate snapshot from runtime + live facts.
// Haxe: AiBase timeReactedLastCommand + speech fallback predicates
pub fn speech_llm_gate_from_runtime(
    rt: &LlmSpeechRuntime,
    speaker_is_human: bool,
    llm_activated: bool,
    ai_age: f32,
    text: &str,
    now_sim: f32,
) -> SpeechLlmGate {
    let never = rt.last_react_sim_time < 1.0;
    let since = if never {
        0.0
    } else {
        (now_sim - rt.last_react_sim_time).max(0.0)
    };
    SpeechLlmGate {
        speaker_is_human,
        llm_activated,
        ai_age,
        text: text.to_string(),
        time_since_last_react_secs: since,
        never_reacted: never,
    }
}

/// Immediate effects when speech gate passes (before async LLM).
// Haxe: AiBase oreally / ally Goto(self) / say "..."
#[derive(Debug, Clone, PartialEq)]
pub struct SpeechLlmStartEffects {
    pub emote_id: i32,
    pub thinking_say: &'static str,
    /// Ally: stop at self + setWaitingTimeMin.
    pub stop_goto_self: bool,
    pub ally_wait_secs: f32,
}

/// `None` if gate fails; otherwise immediate plan (ally stop only when friendly).
// Haxe: AiBase speech fallback ~4973–4990
pub fn plan_speech_llm_start(gate: &SpeechLlmGate, is_ally_friendly: bool) -> Option<SpeechLlmStartEffects> {
    if !should_invoke_llm_for_speech(gate) {
        return None;
    }
    let imm = plan_speech_llm_immediate();
    Some(SpeechLlmStartEffects {
        emote_id: imm.emote_id,
        thinking_say: imm.thinking_say,
        stop_goto_self: imm.stop_and_wait_if_ally && is_ally_friendly,
        ally_wait_secs: imm.ally_wait_secs,
    })
}

/// Post-LLM pure plan: parse + chunks + apply side-effects + ally Goto speaker.
// Haxe: AiHandler.respondToPlayerAsync success + parseAiResponse + sendResponseInChunks
#[derive(Debug, Clone, PartialEq)]
pub struct SpeechLlmCompletePlan {
    pub apply: ApplyAiResponsePlan,
    pub say_chunks: Vec<String>,
    pub wait_secs_per_chunk: Vec<f32>,
    pub record_chat_memory: bool,
    /// Haxe post-say: if ally, Goto(speaker).
    pub goto_speaker: bool,
    /// Full parsed text (for chat memory entry).
    pub reply_text: String,
}

/// Build complete plan after provider returns (or `None` on failure).
// Haxe: respondToPlayerAsync onSuccess path
pub fn plan_speech_llm_complete(
    raw: Option<&str>,
    is_ally_friendly: bool,
    max_len: usize,
    wait_per_100: f32,
) -> SpeechLlmCompletePlan {
    let processed = process_llm_response_for_say(raw, max_len, wait_per_100);
    let apply = plan_apply_parsed_ai_response(&processed.parsed);
    let reply_text = processed.parsed.text.clone();
    let goto_speaker = processed.record_chat_memory && is_ally_friendly && !reply_text.is_empty();
    SpeechLlmCompletePlan {
        apply,
        say_chunks: processed.say_chunks,
        wait_secs_per_chunk: processed.wait_secs_per_chunk,
        record_chat_memory: processed.record_chat_memory,
        goto_speaker,
        reply_text,
    }
}

/// Enqueue chunked SAY: first ready immediately; later chunks after prior waits (Haxe Sys.sleep).
// Haxe: AiHandler.sendResponseInChunks
pub fn enqueue_llm_say_chunks(
    rt: &mut LlmSpeechRuntime,
    chunks: &[String],
    waits: &[f32],
    now: f32,
) {
    let mut t = now;
    for (i, chunk) in chunks.iter().enumerate() {
        if chunk.is_empty() {
            continue;
        }
        rt.pending_says.push_back(PendingLlmSayChunk {
            text: chunk.clone(),
            ready_at: t,
        });
        let w = waits.get(i).copied().unwrap_or(0.0).max(0.0);
        if w > rt.waiting_time_min {
            rt.waiting_time_min = w;
        }
        // Haxe: sleep(waitTime) before next chunk
        t += w;
    }
}

/// Pop one ready chunk, if any.
// Haxe: sendResponseInChunks onSuccess(chunk) over time
pub fn poll_ready_llm_say(rt: &mut LlmSpeechRuntime, now: f32) -> Option<String> {
    if let Some(front) = rt.pending_says.front() {
        if front.ready_at <= now {
            return rt.pending_says.pop_front().map(|c| c.text);
        }
    }
    None
}

/// Haxe `timeReactedLastCommand = TimeHelper.tick` after successful say.
// Haxe: AiBase speech onSuccess timeReactedLastCommand
pub fn mark_llm_reacted(rt: &mut LlmSpeechRuntime, now: f32) {
    rt.last_react_sim_time = now.max(1.0);
}

/// Haxe `setWaitingTimeMin`.
// Haxe: AiBase.setWaitingTimeMin
pub fn set_waiting_time_min(rt: &mut LlmSpeechRuntime, secs: f32) {
    if secs > rt.waiting_time_min {
        rt.waiting_time_min = secs;
    }
}

/// Mark async in-flight flag.
pub fn mark_llm_inflight(rt: &mut LlmSpeechRuntime, v: bool) {
    rt.in_flight = v;
}

// ---------------------------------------------------------------------------
// AI hear fan-out pure (Connection.sendSayToAllClose → each AI)
// ---------------------------------------------------------------------------

/// Snapshot of one player for AI speech hear planning.
#[derive(Debug, Clone)]
pub struct AiSpeechPlayerView {
    pub conn_id: u64,
    pub p_id: i32,
    pub x: i32,
    pub y: i32,
    /// Display first name (uppercase match).
    pub name: String,
    pub is_ai: bool,
    pub age: f32,
}

/// One AI that should process a human SAY (after distance + attention).
#[derive(Debug, Clone, PartialEq)]
pub struct AiSpeechHearer {
    pub conn_id: u64,
    pub p_id: i32,
    pub normalized_text: String,
    pub age: f32,
}

/// Collect AIs that Haxe `AiBase.say` would enter for this speaker/text.
///
/// Skips AI speakers (`player.isAi()`). Distance + attention only — no scripted cmds.
// Haxe: Connection.sendSayToAllClose + AiBase.sayHelper entry
pub fn collect_ai_speech_hearers(
    speaker: &AiSpeechPlayerView,
    text: &str,
    others: &[AiSpeechPlayerView],
    max_dist: f32,
) -> Vec<AiSpeechHearer> {
    // Haxe: if (player.isAi()) return;
    if speaker.is_ai {
        return vec![];
    }

    // Closest living player to speaker within range (Haxe getClosestPlayer).
    let mut closest_id: Option<i32> = None;
    let mut closest_qd = f32::MAX;
    for o in others {
        if o.p_id == speaker.p_id {
            continue;
        }
        let qd = ai_say_quad_distance(speaker.x, speaker.y, o.x, o.y);
        if !ai_within_say_range(qd, max_dist) {
            continue;
        }
        if qd < closest_qd {
            closest_qd = qd;
            closest_id = Some(o.p_id);
        }
    }

    let mut out = Vec::new();
    for o in others {
        if o.p_id == speaker.p_id || !o.is_ai {
            continue;
        }
        let qd = ai_say_quad_distance(speaker.x, speaker.y, o.x, o.y);
        if !ai_within_say_range(qd, max_dist) {
            continue;
        }
        let is_closest = closest_id == Some(o.p_id);
        if let Some(norm) = ai_speech_attention(text, &o.name, is_closest) {
            out.push(AiSpeechHearer {
                conn_id: o.conn_id,
                p_id: o.p_id,
                normalized_text: norm,
                age: o.age,
            });
        }
    }
    out
}

/// Async LLM job snapshot (main-thread prompt; worker fills response).
// Haxe: AiHandler.respondToPlayerAsync thread payload
#[derive(Debug, Clone)]
pub struct LlmSpeechJob {
    pub ai_conn_id: u64,
    pub ai_p_id: i32,
    pub speaker_p_id: i32,
    pub speaker_name: String,
    pub speaker_family: String,
    pub human_message: String,
    pub full_prompt: String,
    pub is_ally: bool,
    pub enqueued_at: f32,
}

/// Completed LLM job ready for sim apply.
#[derive(Debug, Clone)]
pub struct LlmSpeechResult {
    pub ai_conn_id: u64,
    pub ai_p_id: i32,
    pub speaker_p_id: i32,
    pub speaker_name: String,
    pub speaker_family: String,
    pub human_message: String,
    pub raw_response: Option<String>,
    pub is_ally: bool,
}

/// Map a speech job + provider text into a result for sim apply.
// Haxe: respondToPlayerAsync Thread → onSuccess / null on fail
pub fn llm_speech_job_to_result(job: &LlmSpeechJob, raw_response: Option<String>) -> LlmSpeechResult {
    LlmSpeechResult {
        ai_conn_id: job.ai_conn_id,
        ai_p_id: job.ai_p_id,
        speaker_p_id: job.speaker_p_id,
        speaker_name: job.speaker_name.clone(),
        speaker_family: job.speaker_family.clone(),
        human_message: job.human_message.clone(),
        raw_response,
        is_ally: job.is_ally,
    }
}

/// Shared job/result queues between sim and ol-server HTTP drain (**AI-LLM-HTTP-DRAIN**).
// Haxe: AiHandler.respondToPlayerAsync Thread.create payload queues
#[derive(Debug, Default)]
pub struct LlmSpeechIoBridge {
    pub pending_jobs: Vec<LlmSpeechJob>,
    pub completed_results: Vec<LlmSpeechResult>,
}

/// Arc mutex bridge for outer HTTP worker.
pub type LlmSpeechIoShare = Arc<Mutex<LlmSpeechIoBridge>>;

/// Create an empty speech I/O share.
pub fn new_llm_speech_io_share() -> LlmSpeechIoShare {
    Arc::new(Mutex::new(LlmSpeechIoBridge::default()))
}

/// Pull completed results off the share.
pub fn take_completed_llm_results_from_share(share: &LlmSpeechIoShare) -> Vec<LlmSpeechResult> {
    match share.lock() {
        Ok(mut g) => std::mem::take(&mut g.completed_results),
        Err(_) => Vec::new(),
    }
}

/// Push pending jobs onto the share (after sim `take_llm_speech_jobs`).
pub fn push_pending_llm_jobs_to_share(share: &LlmSpeechIoShare, jobs: Vec<LlmSpeechJob>) {
    if jobs.is_empty() {
        return;
    }
    if let Ok(mut g) = share.lock() {
        g.pending_jobs.extend(jobs);
    }
}

/// Drain pending jobs for the HTTP worker.
pub fn take_pending_llm_jobs_from_share(share: &LlmSpeechIoShare) -> Vec<LlmSpeechJob> {
    match share.lock() {
        Ok(mut g) => std::mem::take(&mut g.pending_jobs),
        Err(_) => Vec::new(),
    }
}

/// Push one completed result for sim import.
pub fn push_completed_llm_result_to_share(share: &LlmSpeechIoShare, result: LlmSpeechResult) {
    if let Ok(mut g) = share.lock() {
        g.completed_results.push(result);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_limit_blocks_at_capacity() {
        let mut r = AiCallRateLimit::new();
        let now = 1000.0;
        assert!(r.check_rate_limit(now, 2));
        r.record_call(now);
        assert!(r.check_rate_limit(now + 1.0, 2));
        r.record_call(now + 1.0);
        assert!(!r.check_rate_limit(now + 2.0, 2));
        assert_eq!(r.current_call_count(now + 2.0), 2);
    }

    #[test]
    fn rate_limit_expires_after_hour() {
        let mut r = AiCallRateLimit::new();
        r.record_call(0.0);
        r.record_call(1.0);
        assert_eq!(r.current_call_count(3600.0), 1); // 0.0 cleaned
        assert_eq!(r.current_call_count(3601.1), 0);
    }

    #[test]
    fn is_network_error_patterns() {
        assert!(is_network_error("Connection refused"));
        assert!(is_network_error("HTTP Error: timeout"));
        assert!(!is_network_error("Invalid API key"));
        assert!(!is_network_error("unauthorized"));
        assert!(!is_network_error("rate limit exceeded"));
        assert!(is_network_error("something weird happened"));
    }

    #[test]
    fn chat_response_retries_network_once() {
        let mut rate = AiCallRateLimit::new();
        let mut n = 0;
        let out = chat_response_with(&mut rate, 10.0, 100, || {
            n += 1;
            if n == 1 {
                Err("connection reset".into())
            } else {
                Ok("hello".into())
            }
        });
        assert_eq!(out.attempts, 2);
        assert_eq!(out.response.as_deref(), Some("hello"));
        assert!(!out.rate_limited);
        assert_eq!(rate.raw_len(), 1);
    }

    #[test]
    fn chat_response_no_retry_on_api_error() {
        let mut rate = AiCallRateLimit::new();
        let mut n = 0;
        let out = chat_response_with(&mut rate, 10.0, 100, || {
            n += 1;
            Err("invalid api key".into())
        });
        assert_eq!(out.attempts, 1);
        assert!(out.response.is_none());
    }

    #[test]
    fn chat_response_rate_limited() {
        let mut rate = AiCallRateLimit::new();
        rate.record_call(1.0);
        let out = chat_response_with(&mut rate, 2.0, 1, || Ok("x".into()));
        assert!(out.rate_limited);
        assert_eq!(out.attempts, 0);
        assert!(out.response.is_none());
    }

    #[test]
    fn build_prompt_includes_message_and_command_schema() {
        let parts = PromptParts {
            own_context: "YOU".into(),
            other_context: "OTHER".into(),
            relationship_context: "REL".into(),
            do_command_text: "CMD".into(),
            memory_context: "MEM".into(),
            chat_memory_context: String::new(),
            message: "Hi there".into(),
        };
        let p = build_prompt(&parts);
        assert!(p.starts_with("YOU\nOTHER\nREL\nCMD\nMEM\n"));
        assert!(p.contains("Always respond with valid JSON"));
        assert!(p.contains("Hi there"));
        assert!(!p.contains("\n\nAlways")); // empty chat memory not double-blank only — ok
    }

    #[test]
    fn check_if_should_do_command_branches() {
        assert!(check_if_should_do_command(true, false).contains("follower"));
        assert!(check_if_should_do_command(false, true).contains("close relative"));
        assert!(check_if_should_do_command(false, false).contains("not a follower"));
    }

    #[test]
    fn relationship_info_ally_and_prestige() {
        let v = RelationshipView {
            is_ally: true,
            is_friendly: true,
            to_lost_combat_prestige: 2.0,
            from_lost_combat_prestige: -1.0,
            ..Default::default()
        };
        let t = get_relationship_info(&v);
        assert!(t.contains("allied"));
        assert!(t.contains("friendly"));
        assert!(t.contains("Your combat reputation"));
        assert!(t.contains("Be very careful"));
    }

    #[test]
    fn get_emote_id_table() {
        assert_eq!(get_emote_id("happy"), 0);
        assert_eq!(get_emote_id("JOY"), 5);
        assert_eq!(get_emote_id("snowSplat"), 8);
        assert_eq!(get_emote_id("terrified"), 27);
        assert_eq!(get_emote_id("nope"), -1);
    }

    #[test]
    fn parse_ai_response_json_actions() {
        let raw = r#"{"text":"I will follow you!","emote":"happy","followPlayer":true,"drop":true,"makeItem":"knife"}"#;
        let p = parse_ai_response(raw);
        assert!(p.was_json);
        assert_eq!(p.text, "I will follow you!");
        assert_eq!(p.emote_id, 0);
        assert!(p.actions.follow_player);
        assert!(p.actions.drop);
        assert_eq!(p.actions.make_item.as_deref(), Some("knife"));
    }

    #[test]
    fn parse_ai_response_non_json() {
        let p = parse_ai_response("plain hello");
        assert!(!p.was_json);
        assert_eq!(p.text, "plain hello");
        assert_eq!(p.emote_id, -1);
    }

    #[test]
    fn conversation_log_format() {
        let path = conversation_log_path("log/ai_conversation_log", "2026-07-28");
        assert_eq!(
            path.to_string_lossy(),
            "log/ai_conversation_log_2026-07-28.txt"
        );
        let e = format_conversation_log_entry(
            "2026-07-28 12:00:00",
            path.to_str().unwrap(),
            "PROMPT",
            None,
        );
        assert!(e.contains("--- FULL PROMPT ---"));
        assert!(e.contains("PROMPT"));
        assert!(e.contains("(null - failed or rate limited)"));
    }

    #[test]
    fn format_log_timestamp_from_unix_epoch_day() {
        let (date, ts) = format_log_timestamp_from_unix(0);
        assert_eq!(date, "1970-01-01");
        assert_eq!(ts, "1970-01-01 00:00:00");
        // 2000-01-01 00:00:00 UTC
        let (date, ts) = format_log_timestamp_from_unix(946_684_800);
        assert_eq!(date, "2000-01-01");
        assert_eq!(ts, "2000-01-01 00:00:00");
        let (date2, ts2) = format_log_timestamp_from_unix(946_684_800 + 3661);
        assert_eq!(date2, "2000-01-01");
        assert_eq!(ts2, "2000-01-01 01:01:01");
    }

    #[test]
    fn log_conversation_to_file_writes_entry() {
        let dir = std::env::temp_dir().join(format!(
            "ol_llm_log_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::create_dir_all(&dir);
        let base = dir.join("ai_conversation_log");
        let base_s = base.to_string_lossy().into_owned();
        let path = log_conversation_to_file(
            &base_s,
            "2026-07-28",
            "2026-07-28 12:00:00",
            "FULL PROMPT BODY",
            Some("reply text"),
        )
        .expect("write log");
        let body = std::fs::read_to_string(&path).expect("read log");
        assert!(body.contains("FULL PROMPT BODY"));
        assert!(body.contains("reply text"));
        assert!(body.contains("--- RESPONSE ---"));
        // null path
        let _ = log_conversation_to_file(
            &base_s,
            "2026-07-28",
            "2026-07-28 12:01:00",
            "P2",
            None,
        )
        .expect("write null");
        let body2 = std::fs::read_to_string(&path).expect("read2");
        assert!(body2.contains("(null - failed or rate limited)"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn split_response_at_period() {
        let text = "Hello there friend. More words after the period that continue.";
        let chunks = split_response(text, 20, AI_RESPONSE_SEPARATORS, 10);
        assert!(chunks.len() >= 2);
        assert!(chunks[0].ends_with('.'));
        let joined: String = chunks.concat();
        assert_eq!(joined, text);
    }

    #[test]
    fn plan_response_chunks_short_no_split() {
        let (c, w) = plan_response_chunks("Hi!", 120, 6.0);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0], "Hi!");
        assert!((w[0] - (3.0 / 100.0) * 6.0).abs() < 1e-5);
    }

    #[test]
    fn process_llm_null_no_memory() {
        let r = process_llm_response_for_say(None, 120, 6.0);
        assert!(!r.record_chat_memory);
        assert!(r.say_chunks.is_empty());
    }

    #[test]
    fn process_llm_collapses_newlines_and_parses() {
        let raw = "{\"text\":\"LineA\\nLineB\",\"emote\":\"joy\"}";
        // After collapse of outer newlines; inner \n in JSON string stays until parse
        let r = process_llm_response_for_say(Some(raw), 120, 6.0);
        assert!(r.record_chat_memory);
        assert_eq!(r.parsed.emote_id, 5);
        assert!(!r.say_chunks.is_empty());
    }

    #[test]
    fn is_llm_activated_env_style() {
        assert!(!is_llm_activated(None));
        assert!(!is_llm_activated(Some("")));
        assert!(!is_llm_activated(Some(AI_API_KEY_NOT_SET)));
        assert!(is_llm_activated(Some("sk-test")));
    }

    #[test]
    fn format_date_string_pads() {
        assert_eq!(format_date_string(2026, 7, 8), "2026-07-08");
    }

    #[test]
    fn plan_respond_to_player_sets_ids() {
        let parts = PromptParts {
            own_context: "a".into(),
            other_context: "b".into(),
            relationship_context: "".into(),
            do_command_text: "c".into(),
            memory_context: "".into(),
            chat_memory_context: "".into(),
            message: "yo".into(),
        };
        let plan = plan_respond_to_player(&parts, 7, 9, "Bob", "Clan");
        assert_eq!(plan.from_player_id, 7);
        assert_eq!(plan.to_player_id, 9);
        assert!(plan.full_prompt.contains("yo"));
    }

    #[test]
    fn plan_apply_parsed_emote_and_actions() {
        let parsed = parse_ai_response(
            r#"{"text":"ok","emote":"happy","followPlayer":true,"drop":true,"makeItem":71}"#,
        );
        let plan = plan_apply_parsed_ai_response(&parsed);
        assert_eq!(plan.emote_id, Some(0));
        assert_eq!(plan.emote_seconds, AI_EMOTE_SECONDS);
        assert!(plan.follow_player);
        assert!(plan.drop);
        assert_eq!(plan.make_item.as_deref(), Some("71"));
        assert!(plan.make_item_force);
    }

    #[test]
    fn plan_apply_no_emote_no_make() {
        let parsed = parse_ai_response(r#"{"text":"hi"}"#);
        let plan = plan_apply_parsed_ai_response(&parsed);
        assert_eq!(plan.emote_id, None);
        assert!(!plan.follow_player);
        assert!(plan.make_item.is_none());
        assert!(!plan.make_item_force);
    }

    #[test]
    fn speech_llm_gate_filters() {
        let base = SpeechLlmGate {
            speaker_is_human: true,
            llm_activated: true,
            ai_age: 10.0,
            text: "hello".into(),
            time_since_last_react_secs: 5.0,
            never_reacted: false,
        };
        assert!(should_invoke_llm_for_speech(&base));
        let mut g = base.clone();
        g.speaker_is_human = false;
        assert!(!should_invoke_llm_for_speech(&g));
        g = base.clone();
        g.llm_activated = false;
        assert!(!should_invoke_llm_for_speech(&g));
        g = base.clone();
        g.ai_age = 3.0;
        assert!(!should_invoke_llm_for_speech(&g));
        g = base.clone();
        g.text = "!command".into();
        assert!(!should_invoke_llm_for_speech(&g));
        g = base.clone();
        g.text = "?help".into();
        assert!(!should_invoke_llm_for_speech(&g));
        g = base.clone();
        g.time_since_last_react_secs = 2.0;
        g.never_reacted = false;
        assert!(!should_invoke_llm_for_speech(&g));
        g = base.clone();
        g.time_since_last_react_secs = 0.0;
        g.never_reacted = true;
        assert!(should_invoke_llm_for_speech(&g));
    }

    #[test]
    fn speech_immediate_plan_oreally() {
        let p = plan_speech_llm_immediate();
        assert_eq!(p.emote_id, LLM_SPEECH_ACK_EMOTE_ID);
        assert_eq!(p.thinking_say, "...");
        assert!(p.stop_and_wait_if_ally);
        assert!((p.ally_wait_secs - 6.0).abs() < 1e-5);
    }

    #[test]
    fn check_if_you_are_allied_speech_silent_vs_loud() {
        assert_eq!(
            check_if_you_are_allied_speech(true, false),
            AlliedSpeechOutcome::Allowed
        );
        assert_eq!(
            check_if_you_are_allied_speech(false, true),
            AlliedSpeechOutcome::DeniedSilent
        );
        match check_if_you_are_allied_speech(false, false) {
            AlliedSpeechOutcome::DeniedLoud { say, emote_id } => {
                assert_eq!(say, LLM_NOT_ALLY_SAY);
                assert_eq!(emote_id, LLM_SPEECH_ANGRY_EMOTE_ID);
            }
            other => panic!("expected DeniedLoud, got {other:?}"),
        }
    }

    #[test]
    fn ai_speech_attention_all_name_closest() {
        assert!(ai_speech_attention("hi", "Bob", false).is_none());
        assert_eq!(
            ai_speech_attention("hi", "Bob", true).as_deref(),
            Some("hi")
        );
        assert_eq!(
            ai_speech_attention("ALL hello", "Bob", false).as_deref(),
            Some("hello")
        );
        assert!(ai_speech_attention("help!!", "Bob", false).is_some());
        assert!(ai_speech_attention("what??", "Bob", false).is_some());
        assert!(ai_speech_attention("hey BOB there", "Bob", false).is_some());
    }

    #[test]
    fn ai_within_say_range_max_20() {
        assert!(ai_within_say_range(0.0, MAX_DISTANCE_SAY_AI));
        assert!(ai_within_say_range(400.0, MAX_DISTANCE_SAY_AI)); // 20^2
        assert!(!ai_within_say_range(401.0, MAX_DISTANCE_SAY_AI));
        assert!(
            (ai_say_quad_distance(0, 0, 3, 4) - 25.0).abs() < 1e-5
        );
    }

    #[test]
    fn collect_ai_speech_hearers_skips_ai_speaker_and_far() {
        let speaker = AiSpeechPlayerView {
            conn_id: 1,
            p_id: 10,
            x: 0,
            y: 0,
            name: "Human".into(),
            is_ai: false,
            age: 20.0,
        };
        let near_ai = AiSpeechPlayerView {
            conn_id: 2,
            p_id: 20,
            x: 5,
            y: 0,
            name: "Npc".into(),
            is_ai: true,
            age: 15.0,
        };
        let far_ai = AiSpeechPlayerView {
            conn_id: 3,
            p_id: 30,
            x: 100,
            y: 0,
            name: "Far".into(),
            is_ai: true,
            age: 15.0,
        };
        let human2 = AiSpeechPlayerView {
            conn_id: 4,
            p_id: 40,
            x: 1,
            y: 0,
            name: "Other".into(),
            is_ai: false,
            age: 20.0,
        };
        let others = vec![near_ai, far_ai, human2];
        // Closest is human2 (dist 1) — AI not closest, no ALL → no hearers
        let h = collect_ai_speech_hearers(&speaker, "hello", &others, MAX_DISTANCE_SAY_AI);
        assert!(h.is_empty());
        // Named / ALL reaches near AI even if not closest
        let h2 = collect_ai_speech_hearers(&speaker, "ALL hi", &others, MAX_DISTANCE_SAY_AI);
        assert_eq!(h2.len(), 1);
        assert_eq!(h2[0].p_id, 20);
        assert_eq!(h2[0].normalized_text, "hi");
        // AI speaker → empty
        let mut sp = speaker.clone();
        sp.is_ai = true;
        assert!(collect_ai_speech_hearers(&sp, "ALL hi", &others, MAX_DISTANCE_SAY_AI).is_empty());
    }

    #[test]
    fn plan_speech_llm_start_and_complete_pipeline() {
        let gate = SpeechLlmGate {
            speaker_is_human: true,
            llm_activated: true,
            ai_age: 10.0,
            text: "hello friend".into(),
            time_since_last_react_secs: 5.0,
            never_reacted: false,
        };
        let start = plan_speech_llm_start(&gate, true).expect("start");
        assert_eq!(start.emote_id, LLM_SPEECH_ACK_EMOTE_ID);
        assert!(start.stop_goto_self);
        let start_non = plan_speech_llm_start(&gate, false).expect("start non-ally");
        assert!(!start_non.stop_goto_self);

        let mut g = gate.clone();
        g.llm_activated = false;
        assert!(plan_speech_llm_start(&g, true).is_none());

        let long = "A".repeat(50)
            + ". "
            + &"B".repeat(50)
            + "! "
            + &"C".repeat(50);
        let done = plan_speech_llm_complete(
            Some(&format!(r#"{{"text":"{long}","emote":"happy"}}"#)),
            true,
            MAX_AI_RESPONSE_PER_SAY_DEFAULT,
            AI_WAIT_TIME_PER_100_CHARS_DEFAULT,
        );
        assert!(done.record_chat_memory);
        assert!(done.goto_speaker);
        assert!(done.apply.emote_id.is_some());
        assert!(!done.say_chunks.is_empty());
        assert_eq!(done.say_chunks.len(), done.wait_secs_per_chunk.len());

        let fail = plan_speech_llm_complete(None, true, 120, 6.0);
        assert!(!fail.record_chat_memory);
        assert!(fail.say_chunks.is_empty());
        assert!(!fail.goto_speaker);
    }

    #[test]
    fn llm_speech_runtime_cooldown_and_chunks() {
        let mut rt = LlmSpeechRuntime::default();
        let g0 = speech_llm_gate_from_runtime(&rt, true, true, 10.0, "hi", 100.0);
        assert!(g0.never_reacted);
        assert!(should_invoke_llm_for_speech(&g0));

        mark_llm_reacted(&mut rt, 100.0);
        let g1 = speech_llm_gate_from_runtime(&rt, true, true, 10.0, "hi", 102.0);
        assert!(!g1.never_reacted);
        assert!(!should_invoke_llm_for_speech(&g1)); // 2s < 4s
        let g2 = speech_llm_gate_from_runtime(&rt, true, true, 10.0, "hi", 105.0);
        assert!(should_invoke_llm_for_speech(&g2)); // 5s > 4s

        enqueue_llm_say_chunks(
            &mut rt,
            &["one".into(), "two".into()],
            &[1.0, 2.0],
            200.0,
        );
        assert_eq!(poll_ready_llm_say(&mut rt, 200.0).as_deref(), Some("one"));
        assert!(poll_ready_llm_say(&mut rt, 200.5).is_none());
        assert_eq!(poll_ready_llm_say(&mut rt, 201.0).as_deref(), Some("two"));
        assert!(poll_ready_llm_say(&mut rt, 999.0).is_none());
    }

    #[test]
    fn process_llm_response_long_split_waits() {
        // Long text with separators → multi-chunk + wait_secs_per_chunk
        let body = format!(
            "{}! {}? {}",
            "x".repeat(80),
            "y".repeat(80),
            "z".repeat(80)
        );
        let r = process_llm_response_for_say(
            Some(&body),
            MAX_AI_RESPONSE_PER_SAY_DEFAULT,
            AI_WAIT_TIME_PER_100_CHARS_DEFAULT,
        );
        assert!(r.record_chat_memory);
        assert!(r.say_chunks.len() >= 2);
        for (c, w) in r.say_chunks.iter().zip(r.wait_secs_per_chunk.iter()) {
            let expect = wait_time_for_chars(c.len(), AI_WAIT_TIME_PER_100_CHARS_DEFAULT);
            assert!((w - expect).abs() < 1e-4);
        }
    }

    #[test]
    fn llm_speech_job_to_result_and_io_share() {
        let job = LlmSpeechJob {
            ai_conn_id: 9,
            ai_p_id: 1,
            speaker_p_id: 2,
            speaker_name: "Bob".into(),
            speaker_family: "Clan".into(),
            human_message: "hi".into(),
            full_prompt: "PROMPT".into(),
            is_ally: true,
            enqueued_at: 1.5,
        };
        let ok = llm_speech_job_to_result(&job, Some("hello".into()));
        assert_eq!(ok.ai_conn_id, 9);
        assert_eq!(ok.raw_response.as_deref(), Some("hello"));
        assert!(ok.is_ally);
        let fail = llm_speech_job_to_result(&job, None);
        assert!(fail.raw_response.is_none());

        let share = new_llm_speech_io_share();
        push_pending_llm_jobs_to_share(&share, vec![job.clone()]);
        let taken = take_pending_llm_jobs_from_share(&share);
        assert_eq!(taken.len(), 1);
        assert!(take_pending_llm_jobs_from_share(&share).is_empty());
        push_completed_llm_result_to_share(&share, ok);
        let res = take_completed_llm_results_from_share(&share);
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].human_message, "hi");
    }
}
