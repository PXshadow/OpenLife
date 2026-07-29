//! AI-PROVIDER pure surface (matrix path **S-AIP** / chunk `llm_http`).
//!
//! Canonical implementations live in [`crate::ai_handler`] (shared with AI-HANDLER env helpers).
//! This module re-exports the Haxe `AIProvider` pure API for a stable import path and tests.
//!
//! HTTP I/O is **`ol-server::ai_provider`** (`call_ai` / `call_ai_async`).
//! Secrets: env only — never `server.toml`.

// Haxe: openlife.server.AIProvider (pure) + HTTP-layer aliases
pub use crate::ai_handler::{
    ai_messages_endpoint, ai_messages_endpoint as messages_endpoint, ai_request_headers,
    build_ai_request_body, build_ai_request_body as build_call_request_body,
    ensure_api_key_configured, max_tokens_for_chat_from_env, parse_provider_response,
    resolve_ai_api_url, resolve_ai_model, AI_ANTHROPIC_VERSION,
    AI_ANTHROPIC_VERSION as ANTHROPIC_VERSION, AI_DEFAULT_API_URL,
    AI_DEFAULT_API_URL as AI_API_URL_DEFAULT, AI_DEFAULT_MODEL,
    AI_DEFAULT_MODEL as AI_DEFAULT_MODEL_DEFAULT, AI_MAX_TOKENS_FOR_CHAT_DEFAULT,
    AI_MAX_TOKENS_FOR_CHAT_DEFAULT as AI_MAX_TOKENS_DEFAULT, AI_SYSTEM_DIALOG_PROMPT,
    AI_SYSTEM_DIALOG_PROMPT as AI_SYSTEM_PROMPT,
};

/// Load max tokens with default (Haxe AiMaxTokensForChat = 1024).
pub fn max_tokens_from_env() -> u32 {
    max_tokens_for_chat_from_env().unwrap_or(AI_MAX_TOKENS_FOR_CHAT_DEFAULT)
}

/// Haxe throws when key is sentinel / missing before request.
pub fn missing_key_error() -> String {
    "AI API key not configured. Set ServerSettings.AiApiKey".into()
}

/// Prefix for transport failures (matches Haxe `AI HTTP Error: ` + msg).
pub fn http_error(msg: impl AsRef<str>) -> String {
    format!("AI HTTP Error: {}", msg.as_ref())
}

/// Resolved provider settings for one call (key held only in-process).
#[derive(Debug, Clone)]
pub struct AiProviderConfig {
    pub api_key: String,
    pub api_url: String,
    pub model: String,
    pub max_tokens: u32,
}

impl AiProviderConfig {
    pub fn new(
        api_key: impl Into<String>,
        api_url: Option<&str>,
        model: Option<&str>,
        max_tokens: Option<u32>,
    ) -> Self {
        Self {
            api_key: api_key.into(),
            api_url: resolve_ai_api_url(api_url),
            model: resolve_ai_model(None, model),
            max_tokens: max_tokens.unwrap_or(AI_MAX_TOKENS_DEFAULT),
        }
    }

    pub fn is_configured(&self) -> bool {
        crate::ai_handler::is_llm_activated(Some(&self.api_key))
    }
}

/// Resolve full provider config from env (None if key missing).
pub fn provider_config_from_env() -> Option<AiProviderConfig> {
    let key = crate::ai_handler::api_key_from_env()?;
    Some(AiProviderConfig::new(
        key,
        crate::ai_handler::api_url_from_env().as_deref(),
        crate::ai_handler::default_model_from_env().as_deref(),
        Some(max_tokens_from_env()),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reexport_parse_and_body() {
        let body = build_call_request_body("hi", AI_DEFAULT_MODEL_DEFAULT, 64);
        assert!(body.contains("hi"));
        assert!(body.contains("interactiv"));
        let t = parse_provider_response(
            r#"{"content":[{"type":"text","text":"ok"}]}"#,
        )
        .unwrap();
        assert_eq!(t, "ok");
    }

    #[test]
    fn config_and_http_error() {
        let c = AiProviderConfig::new("sk-x", None, None, None);
        assert!(c.is_configured());
        assert_eq!(c.api_url, AI_API_URL_DEFAULT);
        assert!(http_error("timeout").starts_with("AI HTTP Error: "));
    }
}
