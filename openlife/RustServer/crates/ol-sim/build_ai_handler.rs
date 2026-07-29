//! AI-HANDLER / llm_prompt build-time wire.
//!
//! Inserts `mod ai_handler` + `pub use` into ol-sim `lib.rs`, creates ol-server
//! env-secrets scaffold if missing, and stamps port docs markers.
//! Idempotent.

use std::path::Path;
use std::process::Command;

pub fn ai_handler_wired(lib: &str) -> bool {
    lib.contains("mod ai_handler;")
        && lib.contains("AI-HANDLER")
        && lib.contains("chat_response_with")
        && lib.contains("build_prompt")
        && lib.contains("plan_respond_to_player")
}

fn fix_last_index_of_before(src: &Path) {
    let path = src.join("ai_handler.rs");
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return;
    };
    if !raw.contains("let hay = &s[") {
        return;
    }
    let old = r#"fn last_index_of_before(s: &str, sep: &str, max_len: usize) -> Option<usize> {
    if sep.is_empty() {
        return None;
    }
    let limit = max_len.min(s.len());
    // Search in s[0..=limit] inclusive end for sep that ends within limit+sep? Haxe lastIndexOf
    // with fromIndex: highest index ≤ fromIndex where sep starts.
    let hay = &s[..=limit.min(s.len().saturating_sub(1)).min(s.len())];
    // Safer: scan all matches with start <= max_len
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
    let _ = hay;
    best
}"#;
    let new = r#"fn last_index_of_before(s: &str, sep: &str, max_len: usize) -> Option<usize> {
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
}"#;
    if raw.contains(old) {
        let _ = std::fs::write(&path, raw.replace(old, new));
        return;
    }
    // Fallback: swap whole function for include file.
    if let Some(idx) = raw.find("/// Haxe `lastIndexOf(sep, maxLen)") {
        if let Some(rel) = raw[idx..].find("// ---------------------------------------------------------------------------\n// Env secrets helpers") {
            let rs = idx + rel;
            let mut out = String::new();
            out.push_str(&raw[..idx]);
            out.push_str("include!(\"ai_handler_last_index.inc.rs\");\n\n");
            out.push_str(&raw[rs..]);
            let _ = std::fs::write(&path, out);
        }
    }
}

pub fn patch_ai_handler(src: &Path, workspace: &Path) -> bool {
    fix_last_index_of_before(src);

    let lib_path = src.join("lib.rs");
    let Ok(raw) = std::fs::read_to_string(&lib_path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut t = raw.replace("\r\n", "\n").replace('\r', "\n");

    if !ai_handler_wired(&t) {
        if !t.contains("mod ai_handler;") {
            let anchors = [
                "mod ai_goals;\n// Haxe: Connection.close / ServerAi human-body takeover (AI-TAKEOVER)\nmod ai_takeover;",
                "mod ai_goals;\nmod ai_takeover;",
            ];
            let insert = "mod ai_goals;\n// Haxe: openlife.server.AiHandler rate-limit/prompt/async (AI-HANDLER / S-AIH llm_prompt)\nmod ai_handler;\n// Haxe: Connection.close / ServerAi human-body takeover (AI-TAKEOVER)\nmod ai_takeover;";
            let mut done = false;
            for a in anchors {
                if t.contains(a) {
                    t = t.replacen(a, insert, 1);
                    done = true;
                    break;
                }
            }
            if !done && t.contains("mod ai_goals;") && !t.contains("mod ai_handler;") {
                t = t.replacen(
                    "mod ai_goals;\n",
                    "mod ai_goals;\n// Haxe: openlife.server.AiHandler rate-limit/prompt/async (AI-HANDLER / S-AIH llm_prompt)\nmod ai_handler;\n",
                    1,
                );
            }
        }

        if !t.contains("pub use ai_handler::") && !t.contains("chat_response_with") {
            let pub_block = r#"
// Haxe: AiHandler.hx LLM path (AI-HANDLER / S-AIH llm_prompt) — pure rate limit, prompt, parse, log, chunk
pub use ai_handler::{
    api_key_from_env, api_url_from_env, append_conversation_log, build_prompt,
    chat_response_with, check_if_should_do_command, collapse_response_newlines,
    contains_any_separator, conversation_log_path, default_model_from_env, ensure_log_dir,
    format_conversation_log_entry, format_date_string, get_command_context, get_emote_id,
    get_rate_limit, get_relationship_info, is_llm_activated, is_network_error,
    plan_respond_to_player, plan_response_chunks, parse_ai_response, process_llm_response_for_say,
    split_response, wait_time_for_chars, AiCallRateLimit, AiResponseActions, ChatResponseOutcome,
    ParsedAiResponse, PromptParts, RelationshipView, RespondProcessResult, RespondToPlayerPlan,
    AI_API_KEY_NOT_SET, AI_CALLS_PER_HOUR_DEFAULT, AI_CHAT_MAX_ATTEMPTS,
    AI_CONVERSATION_LOG_BASE_DEFAULT, AI_RATE_WINDOW_SECS, AI_RESPONSE_MAX_SPLITS,
    AI_RESPONSE_SEPARATORS, AI_WAIT_TIME_PER_100_CHARS_DEFAULT, MAX_AI_RESPONSE_PER_SAY_DEFAULT,
};
"#;
            if let Some(idx) = t.find("pub use ai_takeover::{") {
                if let Some(end) = t[idx..].find("\n};\n") {
                    let insert_at = idx + end + "\n};\n".len();
                    t.insert_str(insert_at, pub_block);
                }
            } else if let Some(idx) = t.find("pub use player_soul::{") {
                t.insert_str(idx, pub_block);
            } else if let Some(idx) = t.find("pub use pathfind::{") {
                t.insert_str(idx, pub_block);
            }
        }
    }

    if t.contains("S-AIH AiHandler MISSING") {
        t = t.replace(
            "S-AIH AiHandler MISSING",
            "S-AIH AiHandler pure llm_prompt DONE; AI-PROVIDER HTTP + AI-LLM-WIRE residual",
        );
    }

    let out = if crlf {
        t.replace('\n', "\r\n")
    } else {
        t
    };
    let lib_ok = std::fs::write(&lib_path, out).is_ok();
    let lib_check = std::fs::read_to_string(&lib_path).unwrap_or_default();
    let wired = lib_ok && ai_handler_wired(&lib_check);

    let ol_server_src = workspace.join("crates/ol-server/src");
    let env_mod = ol_server_src.join("ai_llm_env.rs");
    if !env_mod.exists() {
        let inline = r#"//! LLM env secrets loader (AI-HANDLER).
//! Keys never live in `server.toml` / source. Prefer `AI_API_KEY` then `XAI_API_KEY`.

use ol_sim::{
    api_key_from_env, api_url_from_env, default_model_from_env, is_llm_activated,
    AI_API_KEY_NOT_SET, AI_CALLS_PER_HOUR_DEFAULT, AI_CONVERSATION_LOG_BASE_DEFAULT,
    AI_WAIT_TIME_PER_100_CHARS_DEFAULT, MAX_AI_RESPONSE_PER_SAY_DEFAULT,
};

#[derive(Debug, Clone)]
pub struct LlmEnvConfig {
    pub api_key: Option<String>,
    pub api_url: Option<String>,
    pub model: Option<String>,
    pub calls_per_hour: u32,
    pub max_response_per_say: usize,
    pub wait_time_per_100_chars: f32,
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
            log_base: AI_CONVERSATION_LOG_BASE_DEFAULT.to_string(),
        }
    }
}

impl LlmEnvConfig {
    pub fn from_env() -> Self {
        let mut c = Self::default();
        c.api_key = api_key_from_env();
        c.api_url = api_url_from_env();
        c.model = default_model_from_env();
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

    pub fn is_activated(&self) -> bool {
        is_llm_activated(self.api_key.as_deref())
    }

    pub fn debug_status(&self) -> String {
        let key_state = match self.api_key.as_deref() {
            None => "unset",
            Some(k) if k == AI_API_KEY_NOT_SET || k.is_empty() => "not-set",
            Some(_) => "set",
        };
        format!(
            "llm activated={} key={} url={} model={} limit={}/h",
            self.is_activated(),
            key_state,
            self.api_url.as_deref().unwrap_or("(default)"),
            self.model.as_deref().unwrap_or("(default)"),
            self.calls_per_hour
        )
    }
}
"#;
        let _ = std::fs::write(&env_mod, inline);
    }

    let main_path = ol_server_src.join("main.rs");
    if let Ok(main_raw) = std::fs::read_to_string(&main_path) {
        if !main_raw.contains("mod ai_llm_env") {
            let main_crlf = main_raw.contains("\r\n");
            let mut m = main_raw.replace("\r\n", "\n").replace('\r', "\n");
            if m.contains("mod world_boot;") {
                m = m.replacen(
                    "mod world_boot;",
                    "mod world_boot;\n// Haxe: AiHandler env secrets only (AI-HANDLER llm_prompt)\nmod ai_llm_env;",
                    1,
                );
            } else if m.contains("mod selfplay;") {
                m = m.replacen("mod selfplay;", "mod selfplay;\nmod ai_llm_env;", 1);
            }
            let mout = if main_crlf {
                m.replace('\n', "\r\n")
            } else {
                m
            };
            let _ = std::fs::write(&main_path, mout);
        }
    }

    let py = workspace.join("docs/port/_apply_ai_handler_docs.py");
    if py.exists() {
        let _ = Command::new("python")
            .arg(&py)
            .status()
            .or_else(|_| Command::new("python3").arg(&py).status());
    }

    let stamp = src.join(".ai_handler_llm_prompt_patched");
    let _ = std::fs::write(&stamp, b"ai-handler-llm-prompt-1-rs-patched\n");

    if !wired {
        println!("cargo:warning=AI-HANDLER: could not fully wire mod ai_handler into lib.rs");
    }
    wired
}
