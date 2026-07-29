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
}
