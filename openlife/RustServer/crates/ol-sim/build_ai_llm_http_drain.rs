//! Build-time wire for **AI-LLM-HTTP-DRAIN** / `llm_server_drain`.
//!
//! Narrow scope: sim job export/import ↔ ol-server `call_ai_async` drain.
//! Idempotent. Handles CRLF sources. Patches ol-sim + ol-server + docs.

use std::path::Path;

fn normalize_nl(s: &str) -> String {
    s.replace("\r\n", "\n").replace('\r', "\n")
}

fn restore_nl(s: &str, crlf: bool) -> String {
    if crlf {
        s.replace('\n', "\r\n")
    } else {
        s.to_string()
    }
}

fn write_if_changed(path: &Path, crlf: bool, text: &str) -> bool {
    let out = restore_nl(text, crlf);
    let prev = std::fs::read_to_string(path).unwrap_or_default();
    if prev == out {
        return false;
    }
    std::fs::write(path, out).is_ok()
}

/// True when speech HTTP drain is fully wired.
pub fn llm_http_drain_wired(lib: &str, handler: &str, settings: &str, main: &str, provider: &str) -> bool {
    handler.contains("struct LlmSpeechIoBridge")
        && handler.contains("fn llm_speech_job_to_result")
        && handler.contains("fn new_llm_speech_io_share")
        && lib.contains("export_llm_speech_jobs_to_share")
        && lib.contains("import_llm_speech_results_from_share")
        && lib.contains("llm_speech_io")
        && lib.contains("LlmSpeechIoShare")
        && settings.contains("llm_speech_share")
        && main.contains("new_llm_speech_io_share")
        && main.contains("run_llm_speech_http_drain")
        && main.contains("llm_speech_share: Some")
        && provider.contains("run_llm_speech_http_drain")
        && provider.contains("try_drain_params_from_env")
}

/// Patch all surfaces. Returns true when fully ready.
pub fn patch_ai_llm_http_drain(src_dir: &Path, workspace: &Path) -> bool {
    let _ = patch_ai_handler(&src_dir.join("ai_handler.rs"));
    let _ = patch_settings_live(&src_dir.join("settings_live.rs"));
    let _ = patch_lib(&src_dir.join("lib.rs"));
    let _ = patch_ai_provider(&workspace.join("crates/ol-server/src/ai_provider.rs"));
    let _ = patch_main(&workspace.join("crates/ol-server/src/main.rs"));
    patch_docs(workspace);

    let lib = std::fs::read_to_string(src_dir.join("lib.rs")).unwrap_or_default();
    let handler = std::fs::read_to_string(src_dir.join("ai_handler.rs")).unwrap_or_default();
    let settings = std::fs::read_to_string(src_dir.join("settings_live.rs")).unwrap_or_default();
    let main = std::fs::read_to_string(workspace.join("crates/ol-server/src/main.rs")).unwrap_or_default();
    let provider =
        std::fs::read_to_string(workspace.join("crates/ol-server/src/ai_provider.rs")).unwrap_or_default();
    llm_http_drain_wired(&lib, &handler, &settings, &main, &provider)
}

fn patch_ai_handler(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut changed = false;

    if !t.contains("use std::sync::{Arc, Mutex};") {
        let old = "use crate::player_soul::get_combat_prestige_label;\nuse serde_json::Value;\nuse std::path::{Path, PathBuf};\n";
        let new = "use crate::player_soul::get_combat_prestige_label;\nuse serde_json::Value;\nuse std::path::{Path, PathBuf};\nuse std::sync::{Arc, Mutex};\n";
        if t.contains(old) {
            t = t.replacen(old, new, 1);
            changed = true;
        }
    }

    if !t.contains("struct LlmSpeechIoBridge") {
        let old = "    pub raw_response: Option<String>,\n    pub is_ally: bool,\n}\n\n// ---------------------------------------------------------------------------\n// Tests\n// ---------------------------------------------------------------------------\n";
        let new = r#"    pub raw_response: Option<String>,
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
"#;
        if t.contains(old) {
            t = t.replacen(old, new, 1);
            changed = true;
        } else {
            eprintln!("cargo:warning=AI-LLM-HTTP-DRAIN: ai_handler LlmSpeechResult tail anchor missing");
        }
    }

    if !t.contains("llm_speech_job_to_result_and_io_share") {
        let old = "        for (c, w) in r.say_chunks.iter().zip(r.wait_secs_per_chunk.iter()) {\n            let expect = wait_time_for_chars(c.len(), AI_WAIT_TIME_PER_100_CHARS_DEFAULT);\n            assert!((w - expect).abs() < 1e-4);\n        }\n    }\n}\n";
        let new = r#"        for (c, w) in r.say_chunks.iter().zip(r.wait_secs_per_chunk.iter()) {
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
"#;
        if t.contains(old) {
            t = t.replacen(old, new, 1);
            changed = true;
        }
    }

    if changed {
        write_if_changed(path, crlf, &t);
    }
    std::fs::read_to_string(path)
        .map(|s| s.contains("struct LlmSpeechIoBridge"))
        .unwrap_or(false)
}

fn patch_settings_live(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut changed = false;

    if !t.contains("llm_speech_share") {
        let old = "    /// FOODSTATS-DISK: world eaten-food stats for FoodStats.txt autosave dump.\n    pub world_food_share: Option<WorldFoodShare>,\n}\n";
        let new = "    /// FOODSTATS-DISK: world eaten-food stats for FoodStats.txt autosave dump.\n    pub world_food_share: Option<WorldFoodShare>,\n    /// AI-LLM-HTTP-DRAIN: job/result bridge for ol-server `call_ai_async` worker.\n    pub llm_speech_share: Option<crate::LlmSpeechIoShare>,\n}\n";
        if t.contains(old) {
            t = t.replacen(old, new, 1);
            changed = true;
        }
    }
    if !t.contains("llm_speech_share: None") {
        let old = "            war_posse_share: None,\n            players_share: None,\n            world_food_share: None,\n        }\n";
        let new = "            war_posse_share: None,\n            players_share: None,\n            world_food_share: None,\n            llm_speech_share: None,\n        }\n";
        if t.contains(old) {
            t = t.replacen(old, new, 1);
            changed = true;
        }
    }
    if changed {
        write_if_changed(path, crlf, &t);
    }
    std::fs::read_to_string(path)
        .map(|s| s.contains("llm_speech_share"))
        .unwrap_or(false)
}

fn patch_lib(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut changed = false;

    // pub use types
    if !t.contains("LlmSpeechIoShare") {
        let old = "    ApplyAiResponsePlan, ChatResponseOutcome, LlmSpeechJob, LlmSpeechResult, LlmSpeechRuntime,\n    ParsedAiResponse, PendingLlmSayChunk, PromptParts, RelationshipView, RespondProcessResult,\n";
        let new = "    ApplyAiResponsePlan, ChatResponseOutcome, LlmSpeechIoBridge, LlmSpeechIoShare, LlmSpeechJob,\n    LlmSpeechResult, LlmSpeechRuntime, ParsedAiResponse, PendingLlmSayChunk, PromptParts,\n    RelationshipView, RespondProcessResult,\n";
        if t.contains(old) {
            t = t.replacen(old, new, 1);
            changed = true;
        }
    }

    if !t.contains("new_llm_speech_io_share") {
        let old = "    should_invoke_llm_for_speech, speech_llm_gate_from_runtime, split_response, wait_time_for_chars,\n";
        let new = "    llm_speech_job_to_result, new_llm_speech_io_share, push_completed_llm_result_to_share,\n    push_pending_llm_jobs_to_share, should_invoke_llm_for_speech, speech_llm_gate_from_runtime,\n    split_response, take_completed_llm_results_from_share, take_pending_llm_jobs_from_share,\n    wait_time_for_chars,\n";
        if t.contains(old) {
            t = t.replacen(old, new, 1);
            changed = true;
        }
    }

    if !t.contains("pub llm_speech_io:") {
        let old = "    pub llm_speech_jobs: Vec<LlmSpeechJob>,\n    /// AI-LLM-WIRE: completed LLM responses ready for chunk SAY / memory apply.\n    pub llm_speech_results: Vec<LlmSpeechResult>,\n";
        let new = "    pub llm_speech_jobs: Vec<LlmSpeechJob>,\n    /// AI-LLM-WIRE: completed LLM responses ready for chunk SAY / memory apply.\n    pub llm_speech_results: Vec<LlmSpeechResult>,\n    /// AI-LLM-HTTP-DRAIN: optional outer share for ol-server HTTP worker.\n    pub llm_speech_io: Option<LlmSpeechIoShare>,\n";
        if t.contains(old) {
            t = t.replacen(old, new, 1);
            changed = true;
        }
    }

    if !t.contains("llm_speech_io: None") {
        let old = "            llm_speech_jobs: Vec::new(),\n            llm_speech_results: Vec::new(),\n            reputation: ReputationBook::new(),\n";
        let new = "            llm_speech_jobs: Vec::new(),\n            llm_speech_results: Vec::new(),\n            llm_speech_io: None,\n            reputation: ReputationBook::new(),\n";
        if t.contains(old) {
            t = t.replacen(old, new, 1);
            changed = true;
        }
    }

    if !t.contains("import_llm_speech_results_from_share(state)") {
        let old = "fn tick_llm_speech_wire(state: &mut SimState, outbound: &OutboundHub) {\n    // 1) Apply any completed results\n    let results = std::mem::take(&mut state.llm_speech_results);\n";
        let new = "fn tick_llm_speech_wire(state: &mut SimState, outbound: &OutboundHub) {\n    // 0) AI-LLM-HTTP-DRAIN: import HTTP worker results before apply\n    import_llm_speech_results_from_share(state);\n    // 1) Apply any completed results\n    let results = std::mem::take(&mut state.llm_speech_results);\n";
        if t.contains(old) {
            t = t.replacen(old, new, 1);
            changed = true;
        }
    }

    if !t.contains("export_llm_speech_jobs_to_share") {
        let old = r#"    for (cid, p_id, x, y, text) in ready {
        let near = nearby_conn_ids(state, x, y, NEARBY_RANGE);
        send_chat_ps(state, outbound, cid, p_id, &text, &near);
        info!(p_id, text = %text, "sim: AI-LLM-WIRE chunk SAY");
    }
}

/// Drain pending LLM speech jobs (for ol-server HTTP worker).
pub fn take_llm_speech_jobs(state: &mut SimState) -> Vec<LlmSpeechJob> {
    std::mem::take(&mut state.llm_speech_jobs)
}

/// Push a completed LLM result for next `tick_llm_speech_wire`.
pub fn push_llm_speech_result(state: &mut SimState, result: LlmSpeechResult) {
    state.llm_speech_results.push(result);
}
"#;
        let new = r#"    for (cid, p_id, x, y, text) in ready {
        let near = nearby_conn_ids(state, x, y, NEARBY_RANGE);
        send_chat_ps(state, outbound, cid, p_id, &text, &near);
        info!(p_id, text = %text, "sim: AI-LLM-WIRE chunk SAY");
    }
    // 3) AI-LLM-HTTP-DRAIN: export jobs for ol-server call_ai_async worker
    export_llm_speech_jobs_to_share(state);
}

/// Drain pending LLM speech jobs (for ol-server HTTP worker).
pub fn take_llm_speech_jobs(state: &mut SimState) -> Vec<LlmSpeechJob> {
    std::mem::take(&mut state.llm_speech_jobs)
}

/// Push a completed LLM result for next `tick_llm_speech_wire`.
pub fn push_llm_speech_result(state: &mut SimState, result: LlmSpeechResult) {
    state.llm_speech_results.push(result);
}

/// Import completed LLM results from the outer HTTP share into sim queues.
// Haxe: respondToPlayerAsync Thread onSuccess → main apply
pub fn import_llm_speech_results_from_share(state: &mut SimState) {
    let Some(share) = state.llm_speech_io.clone() else {
        return;
    };
    for r in take_completed_llm_results_from_share(&share) {
        push_llm_speech_result(state, r);
    }
}

/// Export pending LLM speech jobs onto the outer HTTP share.
// Haxe: respondToPlayerAsync Thread.create payload handoff
pub fn export_llm_speech_jobs_to_share(state: &mut SimState) {
    let Some(share) = state.llm_speech_io.clone() else {
        return;
    };
    let jobs = take_llm_speech_jobs(state);
    if jobs.is_empty() {
        return;
    }
    let n = jobs.len();
    push_pending_llm_jobs_to_share(&share, jobs);
    info!(n, "sim: AI-LLM-HTTP-DRAIN exported speech jobs");
}
"#;
        if t.contains(old) {
            t = t.replacen(old, new, 1);
            changed = true;
        } else {
            eprintln!("cargo:warning=AI-LLM-HTTP-DRAIN: take/push export anchor missing");
        }
    }

    if !t.contains("llm_speech_share = boot_live") && !t.contains("state.llm_speech_io = Some") {
        let old = "    // FOODSTATS-DISK: WorldFoodStats share for FoodStats.txt autosave dump.\n    let world_food_share = boot_live.as_ref().and_then(|b| b.world_food_share.clone());\n    if let Some(boot) = boot_live {\n";
        let new = "    // FOODSTATS-DISK: WorldFoodStats share for FoodStats.txt autosave dump.\n    let world_food_share = boot_live.as_ref().and_then(|b| b.world_food_share.clone());\n    // AI-LLM-HTTP-DRAIN: outer HTTP job/result share (ol-server drain).\n    let llm_speech_share = boot_live.as_ref().and_then(|b| b.llm_speech_share.clone());\n    if let Some(share) = llm_speech_share {\n        state.llm_speech_io = Some(share);\n        info!(\"sim: AI-LLM-HTTP-DRAIN speech I/O share attached\");\n    }\n    if let Some(boot) = boot_live {\n";
        if t.contains(old) {
            t = t.replacen(old, new, 1);
            changed = true;
        }
    }

    if changed {
        write_if_changed(path, crlf, &t);
    }
    std::fs::read_to_string(path)
        .map(|s| s.contains("export_llm_speech_jobs_to_share"))
        .unwrap_or(false)
}

fn patch_ai_provider(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut changed = false;

    if !t.contains("run_llm_speech_http_drain") {
        // Replace module doc residual + imports
        t = t.replace(
            "//! Speech wire residual: **AI-LLM-WIRE**.\n",
            "//! Speech HTTP drain: **AI-LLM-HTTP-DRAIN** (`take` → `call_ai_async` → `push`).\n",
        );
        t = t.replace(
            "use ol_sim::{\n    ai_messages_endpoint, ai_request_headers, build_ai_request_body, ensure_api_key_configured,\n    is_network_error, parse_provider_response,\n};\nuse tracing::{debug, warn};\n",
            "use ol_sim::{\n    ai_messages_endpoint, ai_request_headers, build_ai_request_body, ensure_api_key_configured,\n    is_network_error, llm_speech_job_to_result, parse_provider_response,\n    push_completed_llm_result_to_share, take_pending_llm_jobs_from_share, AiCallRateLimit,\n    LlmSpeechIoShare, AI_CHAT_MAX_ATTEMPTS,\n};\nuse std::sync::Arc;\nuse std::time::{Duration, SystemTime, UNIX_EPOCH};\nuse tracing::{debug, info, warn};\n",
        );

        let old = r#"fn truncate_for_log(s: &str, max: usize) -> String {
    let t = s.trim();
    if t.len() <= max {
        t.to_string()
    } else {
        format!("{}…", &t[..max])
    }
}

#[cfg(test)]
mod tests {
"#;
        let new = r#"fn truncate_for_log(s: &str, max: usize) -> String {
    let t = s.trim();
    if t.len() <= max {
        t.to_string()
    } else {
        format!("{}…", &t[..max])
    }
}

fn wall_now_secs() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

/// Poll interval for pending speech jobs.
const LLM_DRAIN_POLL_MS: u64 = 50;

/// Max concurrent in-flight HTTP calls (Haxe Thread.create per job; bound here).
const LLM_DRAIN_MAX_INFLIGHT: usize = 4;

/// Haxe `respondToPlayerAsync` worker: drain sim jobs → `call_ai_async` → push results.
// Haxe: AiHandler.respondToPlayerAsync Thread.create + AIProvider.callAi
pub async fn run_llm_speech_http_drain(
    share: LlmSpeechIoShare,
    params: CallAiParams,
    calls_per_hour: u32,
) {
    info!(
        model = %params.model,
        limit = calls_per_hour,
        "AI-LLM-HTTP-DRAIN worker started"
    );
    let mut rate = AiCallRateLimit::new();
    let mut inflight: Vec<tokio::task::JoinHandle<()>> = Vec::new();
    let params = Arc::new(params);

    loop {
        inflight.retain(|h| !h.is_finished());

        if inflight.len() < LLM_DRAIN_MAX_INFLIGHT {
            let mut jobs = take_pending_llm_jobs_from_share(&share);
            if !jobs.is_empty() {
                debug!(n = jobs.len(), "AI-LLM-HTTP-DRAIN took pending jobs");
            }
            while inflight.len() < LLM_DRAIN_MAX_INFLIGHT && !jobs.is_empty() {
                let job = jobs.remove(0);
                let now = wall_now_secs();
                if !rate.check_rate_limit(now, calls_per_hour) {
                    warn!(
                        ai_p_id = job.ai_p_id,
                        "AI-LLM-HTTP-DRAIN rate limited — fail job"
                    );
                    push_completed_llm_result_to_share(
                        &share,
                        llm_speech_job_to_result(&job, None),
                    );
                    continue;
                }
                rate.record_call(now);

                let share_c = Arc::clone(&share);
                let params_c = Arc::clone(&params);
                inflight.push(tokio::spawn(async move {
                    let raw = call_ai_with_retry(&job.full_prompt, params_c.as_ref()).await;
                    if raw.is_some() {
                        info!(
                            ai_p_id = job.ai_p_id,
                            speaker = job.speaker_p_id,
                            "AI-LLM-HTTP-DRAIN call ok"
                        );
                    } else {
                        warn!(
                            ai_p_id = job.ai_p_id,
                            speaker = job.speaker_p_id,
                            "AI-LLM-HTTP-DRAIN call failed"
                        );
                    }
                    push_completed_llm_result_to_share(
                        &share_c,
                        llm_speech_job_to_result(&job, raw),
                    );
                }));
            }
            if !jobs.is_empty() {
                if let Ok(mut g) = share.lock() {
                    let mut rest = std::mem::take(&mut g.pending_jobs);
                    jobs.append(&mut rest);
                    g.pending_jobs = jobs;
                }
            }
        }

        tokio::time::sleep(Duration::from_millis(LLM_DRAIN_POLL_MS)).await;
    }
}

/// Async ChatResponse retry loop (network errors only; max AI_CHAT_MAX_ATTEMPTS).
// Haxe: AiHandler.ChatResponse try/retry
async fn call_ai_with_retry(prompt: &str, params: &CallAiParams) -> Option<String> {
    let mut attempts = 0u32;
    while attempts < AI_CHAT_MAX_ATTEMPTS {
        attempts += 1;
        match call_ai_async(prompt, params).await {
            Ok(text) => return Some(text),
            Err(msg) => {
                let net = is_network_error(&msg);
                warn!(error = %msg, attempt = attempts, net, "AI-LLM-HTTP-DRAIN attempt");
                if !net || attempts >= AI_CHAT_MAX_ATTEMPTS {
                    return None;
                }
            }
        }
    }
    None
}

/// Build drain params from env; `None` when LLM inactive (no key).
pub fn try_drain_params_from_env(env: &crate::ai_llm_env::LlmEnvConfig) -> Option<(CallAiParams, u32)> {
    if !env.is_activated() {
        return None;
    }
    match CallAiParams::from_llm_env(env) {
        Ok(p) => Some((p, env.calls_per_hour)),
        Err(e) => {
            warn!(error = %e, "AI-LLM-HTTP-DRAIN params failed; worker not started");
            None
        }
    }
}

#[cfg(test)]
mod tests {
"#;
        if t.contains(old) {
            t = t.replacen(old, new, 1);
            changed = true;
        } else {
            eprintln!("cargo:warning=AI-LLM-HTTP-DRAIN: ai_provider truncate_for_log anchor missing");
        }
    }

    if !t.contains("try_drain_params_inactive_without_key") {
        t = t.replacen(
            "mod tests {\n    use super::*;\n    use ol_sim::AI_API_KEY_NOT_SET;\n",
            "mod tests {\n    use super::*;\n    use crate::ai_llm_env::LlmEnvConfig;\n    use ol_sim::AI_API_KEY_NOT_SET;\n",
            1,
        );
        let old = r#"    #[test]
    fn http_error_is_network_retryable() {
        let msg = http_error("connection timeout");
        assert!(is_network_error(&msg));
        assert!(msg.starts_with("AI HTTP Error: "));
    }
}
"#;
        let new = r#"    #[test]
    fn http_error_is_network_retryable() {
        let msg = http_error("connection timeout");
        assert!(is_network_error(&msg));
        assert!(msg.starts_with("AI HTTP Error: "));
    }

    #[test]
    fn try_drain_params_inactive_without_key() {
        let env = LlmEnvConfig::default();
        assert!(try_drain_params_from_env(&env).is_none());
    }

    #[test]
    fn try_drain_params_ok_with_key() {
        let env = LlmEnvConfig {
            api_key: Some("sk-test".into()),
            calls_per_hour: 42,
            ..LlmEnvConfig::default()
        };
        let (p, limit) = try_drain_params_from_env(&env).expect("params");
        assert_eq!(p.api_key, "sk-test");
        assert_eq!(limit, 42);
    }

    #[test]
    fn job_to_result_roundtrip_via_share() {
        use ol_sim::{new_llm_speech_io_share, take_completed_llm_results_from_share, LlmSpeechJob};
        let share = new_llm_speech_io_share();
        let job = LlmSpeechJob {
            ai_conn_id: 1,
            ai_p_id: 2,
            speaker_p_id: 3,
            speaker_name: "A".into(),
            speaker_family: "B".into(),
            human_message: "yo".into(),
            full_prompt: "p".into(),
            is_ally: false,
            enqueued_at: 0.0,
        };
        push_completed_llm_result_to_share(
            &share,
            llm_speech_job_to_result(&job, Some("reply".into())),
        );
        let r = take_completed_llm_results_from_share(&share);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].raw_response.as_deref(), Some("reply"));
    }
}
"#;
        if t.contains(old) {
            t = t.replacen(old, new, 1);
            changed = true;
        }
    }

    if changed {
        write_if_changed(path, crlf, &t);
    }
    std::fs::read_to_string(path)
        .map(|s| s.contains("run_llm_speech_http_drain"))
        .unwrap_or(false)
}

fn patch_main(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut changed = false;

    if !t.contains("new_llm_speech_io_share") {
        // Flexible import patch
        if t.contains("load_players, load_war_posse, run_sim_loop_with_views,") {
            t = t.replacen(
                "load_players, load_war_posse, run_sim_loop_with_views,",
                "load_players, load_war_posse, new_llm_speech_io_share, run_sim_loop_with_views,",
                1,
            );
            changed = true;
        }
    }

    if !t.contains("let llm_speech_share = new_llm_speech_io_share()") {
        let old = "    let llm_env = ai_llm_env::LlmEnvConfig::from_env();\n    info!(status = %llm_env.debug_status(), \"LLM env loaded\");\n\n    let counters = Arc::new(Counters::new());\n";
        let new = "    let llm_env = ai_llm_env::LlmEnvConfig::from_env();\n    info!(status = %llm_env.debug_status(), \"LLM env loaded\");\n    // AI-LLM-HTTP-DRAIN: shared job/result queues (sim ↔ HTTP worker)\n    let llm_speech_share = new_llm_speech_io_share();\n\n    let counters = Arc::new(Counters::new());\n";
        if t.contains(old) {
            t = t.replacen(old, new, 1);
            changed = true;
        }
    }

    if !t.contains("llm_speech_share: Some") {
        let old = "            // FOODSTATS-DISK: FoodStats.txt autosave mirror\n            world_food_share: Some(Arc::clone(&shared_world_food)),\n        };\n";
        let new = "            // FOODSTATS-DISK: FoodStats.txt autosave mirror\n            world_food_share: Some(Arc::clone(&shared_world_food)),\n            // AI-LLM-HTTP-DRAIN: speech job/result bridge for call_ai_async worker\n            llm_speech_share: Some(Arc::clone(&llm_speech_share)),\n        };\n";
        if t.contains(old) {
            t = t.replacen(old, new, 1);
            changed = true;
        }
    }

    if !t.contains("run_llm_speech_http_drain") {
        let old = "    // AI NPC scheduler + activity log (RAM ring, flush every 30s).\n    let npc_activity = Arc::new(npc_activity::NpcActivityLog::new(\n";
        let new = r#"    // AI-LLM-HTTP-DRAIN: Haxe respondToPlayerAsync Thread → callAi → result apply
    if let Some((params, limit)) = ai_provider::try_drain_params_from_env(&llm_env) {
        let share = Arc::clone(&llm_speech_share);
        handles.push(tokio::spawn(async move {
            ai_provider::run_llm_speech_http_drain(share, params, limit).await;
        }));
        info!("AI-LLM-HTTP-DRAIN worker spawned");
    } else {
        info!("AI-LLM-HTTP-DRAIN idle (LLM inactive / no AI_API_KEY)");
    }

    // AI NPC scheduler + activity log (RAM ring, flush every 30s).
    let npc_activity = Arc::new(npc_activity::NpcActivityLog::new(
"#;
        if t.contains(old) {
            t = t.replacen(old, new, 1);
            changed = true;
        }
    }

    if changed {
        write_if_changed(path, crlf, &t);
    }
    std::fs::read_to_string(path)
        .map(|s| s.contains("run_llm_speech_http_drain") && s.contains("new_llm_speech_io_share"))
        .unwrap_or(false)
}

fn patch_docs(workspace: &Path) {
    // FILE_MATRIX
    let p = workspace.join("docs/port/FILE_MATRIX.md");
    if let Ok(raw) = std::fs::read_to_string(&p) {
        let crlf = raw.contains("\r\n");
        let mut t = normalize_nl(&raw);
        if !t.contains("AI-LLM-HTTP-DRAIN") {
            t = t.replacen(
                "| **AI-LLM-WIRE** | `AiBase.say`/`sayHelper` + `AiHandler.respondToPlayerAsync` | — | `ol-sim/ai_handler.rs` + `Player.llm_speech` + `lib` fan-out/tick | **PARTIAL → speech core DONE** | Pure hear/attention/ally/gate/start/complete/chunks + live free-form SAY→AI fan-out (oreally/`...`/enqueue job) + tick apply results/chunk SAY + chat memory; residual: ol-server HTTP drain of `llm_speech_jobs`, scripted sayHelper cmds, follow/drop/make live apply, ally Goto pathfind |\n",
                "| **AI-LLM-WIRE** | `AiBase.say`/`sayHelper` + `AiHandler.respondToPlayerAsync` | — | `ol-sim/ai_handler.rs` + `Player.llm_speech` + `lib` fan-out/tick | **PARTIAL → speech core DONE** | Pure hear/attention/ally/gate/start/complete/chunks + live free-form SAY→AI fan-out + tick apply/chunk SAY + chat memory; residual: scripted cmds, follow/drop/make live apply, ally Goto pathfind |\n| **AI-LLM-HTTP-DRAIN** | `AiHandler.respondToPlayerAsync` Thread + `AIProvider.callAi` | — | `ol-server/ai_provider` drain + `main` + `LlmSpeechIoShare` | **DONE** (llm_server_drain) | export/import share + `run_llm_speech_http_drain` take→`call_ai_async`→push; env secrets only; residual apply → **AI-LLM-APPLY** |\n",
                1,
            );
            let _ = write_if_changed(&p, crlf, &t);
        }
    }

    // TODO_PORT
    let p = workspace.join("docs/port/TODO_PORT.md");
    if let Ok(raw) = std::fs::read_to_string(&p) {
        let crlf = raw.contains("\r\n");
        let mut t = normalize_nl(&raw);
        if !t.contains("AI-LLM-HTTP-DRAIN llm_server_drain DONE") {
            t = t.replacen(
                "Last updated: **2026-07-28** (AI-HANDLER llm_prompt) (AI-PROVIDER llm_http) (FOODSTATS-DISK foodstats_txt) (NOOB-NOBLE-SPAWN spawn_weights)",
                "Last updated: **2026-07-28** (AI-LLM-HTTP-DRAIN llm_server_drain) (AI-HANDLER llm_prompt) (AI-PROVIDER llm_http)",
                1,
            );
            t = t.replace(
                "| LLM AiHandler / AIProvider | ■■ | ■ | | **AI-HANDLER** pure + **AI-PROVIDER** HTTP DONE; **AI-LLM-WIRE** speech hear/gate/chunk core DONE; residual: HTTP job drain + scripted cmds + action apply |",
                "| LLM AiHandler / AIProvider | ■■ | ■ | | **AI-HANDLER** pure + **AI-PROVIDER** HTTP DONE; **AI-LLM-WIRE** speech core DONE; **AI-LLM-HTTP-DRAIN** DONE; residual: scripted cmds + action apply |",
            );
            t = t.replace(
                "S-AIH+S-AIP llm pure+HTTP DONE; **AI-LLM-WIRE** speech core DONE (HTTP drain residual)",
                "S-AIH+S-AIP llm pure+HTTP DONE; **AI-LLM-WIRE** + **AI-LLM-HTTP-DRAIN** DONE (apply residual)",
            );
            let old = "- [~] **AI-LLM-WIRE speech_llm PARTIAL → core DONE** — pure hear/attention/`check_if_you_are_allied_speech`/gate/start/complete/chunk runtime; `Player.llm_speech`; free-form SAY fan-out + tick chunk SAY + chat memory + job/result queues; residual: ol-server drain `take_llm_speech_jobs`→`call_ai`→`push_llm_speech_result`; live `ApplyAiResponsePlan` (emote/follow/drop/makeItem); live `RelationshipView` from ally/exile/leader/prestige; conversation log + `toSoul.addChatEntry` after live LLM; AiBase scripted cmds (HOLA/FOLLOW/…); ally Goto pathfind; Haxe `checkIfShouldDoCommand` exile branch TODO both sides\n";
            let new = "- [x] **AI-LLM-WIRE speech_llm core DONE** — pure hear/attention/gate/start/complete/chunk + live fan-out + tick apply/chunk SAY + job queues. Residual apply/scripted → **AI-LLM-APPLY**\n- [x] **AI-LLM-HTTP-DRAIN llm_server_drain DONE** — `LlmSpeechIoShare` + sim export/import + ol-server `run_llm_speech_http_drain` (take→`call_ai_async` rate-limit/retry→push); env only. Residual: live ApplyAiResponsePlan → **AI-LLM-APPLY**; scripted cmds; ally Goto; conversation log file\n";
            if t.contains(old) {
                t = t.replacen(old, new, 1);
            }
            let _ = write_if_changed(&p, crlf, &t);
        }
    }

    // QUEUE
    let p = workspace.join("docs/port/QUEUE.md");
    if let Ok(raw) = std::fs::read_to_string(&p) {
        let crlf = raw.contains("\r\n");
        let mut t = normalize_nl(&raw);
        t = t.replace(
            "| `AI-LLM-HTTP-DRAIN` | llm_server_drain | **Narrow** ol-server drain: take_llm_speech_jobs → call_ai → push |\n",
            "| ~~`AI-LLM-HTTP-DRAIN`~~ | llm_server_drain | **DONE** take→call_ai_async→push |\n",
        );
        t = t.replace(
            "**AI-HANDLER** PARTIAL (pure path + env) · **AI-LLM-WIRE** PARTIAL (speech jobs; drain residual)",
            "**AI-LLM-HTTP-DRAIN** DONE · **AI-HANDLER** PARTIAL · **AI-LLM-WIRE** speech core DONE",
        );
        let _ = write_if_changed(&p, crlf, &t);
    }

    // CALL_INDEX
    let p = workspace.join("docs/port/CALL_INDEX.md");
    if let Ok(raw) = std::fs::read_to_string(&p) {
        let crlf = raw.contains("\r\n");
        let mut t = normalize_nl(&raw);
        if !t.contains("run_llm_speech_http_drain") {
            t = t.replacen(
                "| job queue | `take_llm_speech_jobs` / `push_llm_speech_result` / `tick_llm_speech_wire` | HTTP residual drains jobs |\n",
                "| job queue | `take_llm_speech_jobs` / `push_llm_speech_result` / `tick_llm_speech_wire` | apply path |\n| HTTP drain | `export_llm_speech_jobs_to_share` / `import_llm_speech_results_from_share` / `LlmSpeechIoShare` | sim↔server |\n| `run_llm_speech_http_drain` | `ol-server::ai_provider::run_llm_speech_http_drain` | take→`call_ai_async`→push |\n| job→result | `llm_speech_job_to_result` | pure map |\n",
                1,
            );
            t = t.replace(
                "| Residual | — | HTTP drain; live ApplyAiResponsePlan; live RelationshipView; log/`addChatEntry`; scripted cmds; ally Goto; exile-branch TODO |\n",
                "| Residual | — | live ApplyAiResponsePlan (**AI-LLM-APPLY**); live RelationshipView; log file; scripted cmds; ally Goto; exile-branch TODO |\n",
            );
            let _ = write_if_changed(&p, crlf, &t);
        }
    }

    // changelog
    let ch = workspace.join("docs/port/changelog/2026-07-28-AI-LLM-HTTP-DRAIN.md");
    if !ch.exists() {
        let body = r#"# AI-LLM-HTTP-DRAIN / llm_server_drain

**Date:** 2026-07-28  
**Mode:** implement  
**Status:** DONE

## Scope (narrow)

`export_llm_speech_jobs_to_share` → `take_pending` → `call_ai_async` → `push_completed` → `import` → `tick_llm_speech_wire`

## Rust

- `LlmSpeechIoShare` + `llm_speech_job_to_result` (`ai_handler.rs`)
- Sim: `SimState.llm_speech_io`, export/import around `tick_llm_speech_wire`, `SimBootLive.llm_speech_share`
- ol-server: `run_llm_speech_http_drain` + main spawn when `AI_API_KEY` set

## Secrets

Env only. Never `server.toml`.

## Verify

```powershell
cd C:\OhOl\OpenLife\openlife\RustServer
$env:PATH = "$env:USERPROFILE\.cargo\bin;" + $env:PATH
cargo test -p ol-sim --lib -- llm_speech_job_to_result -- --test-threads=1
cargo test -p ol-server -- try_drain -- --test-threads=1
```
"#;
        let _ = std::fs::write(ch, body);
    }

    // AI-LLM-WIRE residual note
    let p = workspace.join("docs/port/changelog/2026-07-28-AI-LLM-WIRE.md");
    if let Ok(raw) = std::fs::read_to_string(&p) {
        let crlf = raw.contains("\r\n");
        let mut t = normalize_nl(&raw);
        t = t.replace(
            "**Status:** PARTIAL — speech hear/gate/chunk core DONE; HTTP job drain residual",
            "**Status:** PARTIAL — speech hear/gate/chunk core DONE; HTTP drain → **AI-LLM-HTTP-DRAIN DONE**",
        );
        t = t.replace(
            "- ol-server drain: take jobs → `call_ai` / `call_ai_async` → push results\n",
            "- ~~ol-server drain~~ → **AI-LLM-HTTP-DRAIN DONE**\n",
        );
        let _ = write_if_changed(&p, crlf, &t);
    }
}
