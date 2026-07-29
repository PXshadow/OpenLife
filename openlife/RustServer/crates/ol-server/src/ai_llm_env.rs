//! LLM env secrets loader (AI-HANDLER + AI-PROVIDER).
//!
//! Keys never live in `server.toml` / source. Prefer `AI_API_KEY` then `XAI_API_KEY`.
//! Pure request body + `parse_provider_response` live in `ol-sim::ai_handler`.
//! Live HTTP: **AI-PROVIDER** (`ol-server::ai_provider`). Speech tick wire: **AI-LLM-WIRE**.

use ol_sim::{
    api_key_from_env, api_url_from_env, default_model_from_env, is_llm_activated,
    max_tokens_for_chat_from_env, resolve_ai_api_url, resolve_ai_model, AI_API_KEY_NOT_SET,
    AI_CALLS_PER_HOUR_DEFAULT, AI_CONVERSATION_LOG_BASE_DEFAULT, AI_MAX_TOKENS_FOR_CHAT_DEFAULT,
    AI_WAIT_TIME_PER_100_CHARS_DEFAULT, MAX_AI_RESPONSE_PER_SAY_DEFAULT,
};

/// Operator-facing LLM config resolved from process environment only.
#[derive(Debug, Clone)]
pub struct LlmEnvConfig {
    pub api_key: Option<String>,
    /// Raw env `AI_API_URL` (None → use Haxe default via `resolved_api_url`).
    pub api_url: Option<String>,
    /// Raw env `AI_DEFAULT_MODEL`.
    pub model: Option<String>,
    pub calls_per_hour: u32,
    pub max_response_per_say: usize,
    pub wait_time_per_100_chars: f32,
    /// Haxe `ServerSettings.AiMaxTokensForChat`.
    pub max_tokens_for_chat: u32,
    pub log_base: String,
}

impl Default for LlmEnvConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            api_url: None,
            model: None,
            calls_per_hour: AI_CALLS_PER_HOUR_DEFAULT,
            max_response_per_say: MAX_AI_RESPONSE_PER_SAY_DEFAULT,
            wait_time_per_100_chars: AI_WAIT_TIME_PER_100_CHARS_DEFAULT,
            max_tokens_for_chat: AI_MAX_TOKENS_FOR_CHAT_DEFAULT,
            log_base: AI_CONVERSATION_LOG_BASE_DEFAULT.to_string(),
        }
    }
}

impl LlmEnvConfig {
    /// Load from env. Never panics; missing key ⇒ LLM inactive.
    // Haxe: ServerSettings.AiApi* + AIProvider.IsLLMActivated
    pub fn from_env() -> Self {
        let mut c = Self::default();
        c.api_key = api_key_from_env();
        c.api_url = api_url_from_env();
        c.model = default_model_from_env();
        if let Some(n) = max_tokens_for_chat_from_env() {
            c.max_tokens_for_chat = n;
        }
        if let Ok(v) = std::env::var("AI_CALLS_PER_HOUR") {
            if let Ok(n) = v.parse::<u32>() {
                c.calls_per_hour = n;
            }
        }
        if let Ok(v) = std::env::var("AI_MAX_RESPONSE_PER_SAY") {
            if let Ok(n) = v.parse::<usize>() {
                c.max_response_per_say = n;
            }
        }
        if let Ok(v) = std::env::var("AI_WAIT_TIME_PER_100_CHARS") {
            if let Ok(n) = v.parse::<f32>() {
                c.wait_time_per_100_chars = n;
            }
        }
        if let Ok(v) = std::env::var("AI_CONVERSATION_LOG_BASE") {
            if !v.is_empty() {
                c.log_base = v;
            }
        }
        c
    }

    /// Haxe `AIProvider.IsLLMActivated`.
    pub fn is_activated(&self) -> bool {
        is_llm_activated(self.api_key.as_deref())
    }

    /// Effective API base URL (env or MiniMax Anthropic default).
    pub fn resolved_api_url(&self) -> String {
        resolve_ai_api_url(self.api_url.as_deref())
    }

    /// Effective model name (env or Haxe default).
    pub fn resolved_model(&self) -> String {
        resolve_ai_model(None, self.model.as_deref())
    }

    /// Debug display that never prints the raw key.
    pub fn debug_status(&self) -> String {
        let key_state = match self.api_key.as_deref() {
            None => "unset",
            Some(k) if k == AI_API_KEY_NOT_SET || k.is_empty() => "not-set",
            Some(_) => "set",
        };
        format!(
            "llm activated={} key={} url={} model={} limit={}/h max_tokens={}",
            self.is_activated(),
            key_state,
            self.resolved_api_url(),
            self.resolved_model(),
            self.calls_per_hour,
            self.max_tokens_for_chat
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_inactive_without_key() {
        let c = LlmEnvConfig::default();
        assert!(!c.is_activated());
        assert_eq!(c.max_tokens_for_chat, AI_MAX_TOKENS_FOR_CHAT_DEFAULT);
        assert!(c.resolved_api_url().contains("minimax"));
        assert!(!c.resolved_model().is_empty());
    }

    #[test]
    fn debug_status_never_embeds_secret() {
        let mut c = LlmEnvConfig::default();
        c.api_key = Some("sk-super-secret-value-xyz".into());
        let s = c.debug_status();
        assert!(!s.contains("sk-super-secret"));
        assert!(s.contains("key=set"));
        assert!(s.contains("activated=true"));
    }
}
