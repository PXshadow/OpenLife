//! AIProvider HTTP client — MiniMax / Anthropic-compatible `/v1/messages`.
//!
//! Haxe: `openlife.server.AIProvider.callAi` (matrix **AI-PROVIDER**, chunk `llm_http`).
//!
//! Pure body/parse/headers: `ol_sim` (`build_ai_request_body`, `parse_provider_response`, …).
//! Secrets: env only via [`crate::ai_llm_env::LlmEnvConfig`] — never `server.toml`.
//! Speech HTTP drain: **AI-LLM-HTTP-DRAIN** (`take` → `call_ai_async` → `push`).

use ol_sim::{
    ai_messages_endpoint, ai_request_headers, build_ai_request_body, ensure_api_key_configured,
    is_network_error, llm_speech_job_to_result, log_conversation_to_file_now,
    parse_provider_response, push_completed_llm_result_to_share, take_pending_llm_jobs_from_share,
    AiCallRateLimit, LlmSpeechIoShare, AI_CHAT_MAX_ATTEMPTS, AI_CONVERSATION_LOG_BASE_DEFAULT,
};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn};

use crate::ai_llm_env::LlmEnvConfig;

/// Default HTTP timeout for a single LLM call (Haxe blocks; keep bounded).
const CALL_TIMEOUT_SECS: u64 = 120;

/// Resolved call parameters (no secret dump helpers here).
#[derive(Debug, Clone)]
pub struct CallAiParams {
    pub api_key: String,
    pub api_url: String,
    pub model: String,
    pub max_tokens: u32,
}

impl CallAiParams {
    /// From boot `LlmEnvConfig` (resolved URL/model + key check).
    pub fn from_llm_env(env: &LlmEnvConfig) -> Result<Self, String> {
        let key = ensure_api_key_configured(env.api_key.as_deref())?.to_string();
        Ok(Self {
            api_key: key,
            api_url: env.resolved_api_url(),
            model: env.resolved_model(),
            max_tokens: env.max_tokens_for_chat,
        })
    }

    pub fn from_env() -> Result<Self, String> {
        Self::from_llm_env(&LlmEnvConfig::from_env())
    }
}

/// Prefix transport failures like Haxe `AI HTTP Error: ` + msg (retryable via `is_network_error`).
fn http_error(msg: impl AsRef<str>) -> String {
    format!("AI HTTP Error: {}", msg.as_ref())
}

/// Haxe `AIProvider.callAi` — async POST to `{url}/v1/messages`.
// Haxe: AIProvider.callAi
pub async fn call_ai_async(prompt: &str, params: &CallAiParams) -> Result<String, String> {
    let _ = ensure_api_key_configured(Some(params.api_key.as_str()))?;

    let url = ai_messages_endpoint(&params.api_url);
    let body = build_ai_request_body(prompt, &params.model, params.max_tokens);
    let headers = ai_request_headers(&params.api_key);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(CALL_TIMEOUT_SECS))
        .build()
        .map_err(|e| http_error(e.to_string()))?;

    debug!(
        url = %url,
        model = %params.model,
        max_tokens = params.max_tokens,
        prompt_chars = prompt.len(),
        "AIProvider.call_ai POST"
    );

    let mut req = client.post(&url).body(body);
    for (k, v) in headers {
        req = req.header(k, v);
    }

    let resp = req.send().await.map_err(|e| {
        warn!(error = %e, "AIProvider HTTP transport error");
        http_error(e.to_string())
    })?;

    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| http_error(format!("body: {e}")))?;

    if text.trim().is_empty() {
        return Err("AI empty response".into());
    }

    if !status.is_success() {
        // Prefer structured API error message when body is error JSON.
        match parse_provider_response(&text) {
            Ok(ok_text) => return Ok(ok_text),
            Err(api_msg)
                if api_msg != "AI response format not recognized"
                    && !api_msg.starts_with("Failed to parse") =>
            {
                return Err(api_msg);
            }
            Err(_) => {
                return Err(http_error(format!(
                    "status {status}: {}",
                    truncate_for_log(&text, 200)
                )));
            }
        }
    }

    parse_provider_response(&text)
}

/// Blocking `callAi` (Haxe synchronous `http.request(true)`).
///
/// Prefer [`call_ai_async`] from async contexts. Safe for `chat_response_with` inject.
// Haxe: AIProvider.callAi (blocking)
pub fn call_ai(prompt: &str, params: &CallAiParams) -> Result<String, String> {
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => tokio::task::block_in_place(|| handle.block_on(call_ai_async(prompt, params))),
        Err(_) => {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| http_error(format!("runtime: {e}")))?;
            rt.block_on(call_ai_async(prompt, params))
        }
    }
}

/// One-shot env-based call (for injects / smoke).
pub fn call_ai_from_env(prompt: &str) -> Result<String, String> {
    let params = CallAiParams::from_env()?;
    call_ai(prompt, &params)
}

/// `chat_response_with` inject using a cloned params snapshot.
pub fn make_call_ai_inject(
    params: CallAiParams,
    prompt: String,
) -> impl FnMut() -> Result<String, String> {
    move || call_ai(&prompt, &params)
}

fn truncate_for_log(s: &str, max: usize) -> String {
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

/// Best-effort Haxe `logToFile` on the drain worker (never blocks apply).
// Haxe: AiHandler.logToFile after ChatResponse
fn drain_log_conversation(log_base: &str, full_prompt: &str, response: Option<&str>) {
    if let Err(e) = log_conversation_to_file_now(log_base, full_prompt, response) {
        warn!(error = %e, "AI-LLM-HTTP-DRAIN logToFile failed");
    }
}

/// Haxe `respondToPlayerAsync` worker: drain sim jobs → `call_ai_async` → log → push results.
// Haxe: AiHandler.respondToPlayerAsync Thread.create + AIProvider.callAi + logToFile
pub async fn run_llm_speech_http_drain(
    share: LlmSpeechIoShare,
    params: CallAiParams,
    calls_per_hour: u32,
    log_base: String,
) {
    info!(
        model = %params.model,
        limit = calls_per_hour,
        log_base = %log_base,
        "AI-LLM-HTTP-DRAIN worker started"
    );
    let mut rate = AiCallRateLimit::new();
    let mut inflight: Vec<tokio::task::JoinHandle<()>> = Vec::new();
    let params = Arc::new(params);
    let log_base = Arc::new(log_base);

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
                    // Haxe ChatResponse null → log still designed for null response
                    drain_log_conversation(log_base.as_str(), &job.full_prompt, None);
                    push_completed_llm_result_to_share(
                        &share,
                        llm_speech_job_to_result(&job, None),
                    );
                    continue;
                }
                rate.record_call(now);

                let share_c = Arc::clone(&share);
                let params_c = Arc::clone(&params);
                let log_c = Arc::clone(&log_base);
                inflight.push(tokio::spawn(async move {
                    let raw = call_ai_with_retry(&job.full_prompt, params_c.as_ref()).await;
                    // Haxe: collapse newlines then logToFile(fullPrompt, response)
                    let logged = raw.as_deref().map(ol_sim::collapse_response_newlines);
                    drain_log_conversation(
                        log_c.as_str(),
                        &job.full_prompt,
                        logged.as_deref(),
                    );
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
                    // Apply path still gets original (un-collapsed) raw; plan collapses again
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
///
/// Returns `(CallAiParams, calls_per_hour, log_base)` for `run_llm_speech_http_drain`.
pub fn try_drain_params_from_env(
    env: &crate::ai_llm_env::LlmEnvConfig,
) -> Option<(CallAiParams, u32, String)> {
    if !env.is_activated() {
        return None;
    }
    match CallAiParams::from_llm_env(env) {
        Ok(p) => {
            let log_base = if env.log_base.is_empty() {
                AI_CONVERSATION_LOG_BASE_DEFAULT.to_string()
            } else {
                env.log_base.clone()
            };
            Some((p, env.calls_per_hour, log_base))
        }
        Err(e) => {
            warn!(error = %e, "AI-LLM-HTTP-DRAIN params failed; worker not started");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_llm_env::LlmEnvConfig;
    use ol_sim::AI_API_KEY_NOT_SET;

    #[test]
    fn params_reject_missing_key() {
        let env = LlmEnvConfig {
            api_key: None,
            ..LlmEnvConfig::default()
        };
        let e = CallAiParams::from_llm_env(&env).unwrap_err();
        assert!(e.contains("API key") || e.contains("AI_API_KEY"));
    }

    #[test]
    fn params_reject_sentinel() {
        let env = LlmEnvConfig {
            api_key: Some(AI_API_KEY_NOT_SET.to_string()),
            ..LlmEnvConfig::default()
        };
        assert!(CallAiParams::from_llm_env(&env).is_err());
    }

    #[test]
    fn params_ok_resolves_defaults() {
        let env = LlmEnvConfig {
            api_key: Some("sk-test".into()),
            api_url: None,
            model: None,
            max_tokens_for_chat: 256,
            ..LlmEnvConfig::default()
        };
        let p = CallAiParams::from_llm_env(&env).unwrap();
        assert_eq!(p.api_key, "sk-test");
        assert!(p.api_url.contains("minimax"));
        assert!(!p.model.is_empty());
        assert_eq!(p.max_tokens, 256);
    }

    #[test]
    fn call_ai_missing_key_no_http() {
        let params = CallAiParams {
            api_key: AI_API_KEY_NOT_SET.to_string(),
            api_url: "https://example.test".into(),
            model: "m".into(),
            max_tokens: 64,
        };
        let e = call_ai("hi", &params).unwrap_err();
        assert!(e.contains("API key") || e.contains("AI_API_KEY"));
    }

    #[test]
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
            log_base: "log/custom_ai_log".into(),
            ..LlmEnvConfig::default()
        };
        let (p, limit, log_base) = try_drain_params_from_env(&env).expect("params");
        assert_eq!(p.api_key, "sk-test");
        assert_eq!(limit, 42);
        assert_eq!(log_base, "log/custom_ai_log");
    }

    #[test]
    fn try_drain_params_default_log_base() {
        let env = LlmEnvConfig {
            api_key: Some("sk-test".into()),
            ..LlmEnvConfig::default()
        };
        let (_, _, log_base) = try_drain_params_from_env(&env).expect("params");
        assert_eq!(log_base, AI_CONVERSATION_LOG_BASE_DEFAULT);
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
