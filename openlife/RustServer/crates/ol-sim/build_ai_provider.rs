//! Build-time wire for **AI-PROVIDER** / `llm_http`.
//!
//! Pure helpers already ship in `ai_handler.rs` (+ thin `ai_provider` re-export).
//! Ensures `mod ai_provider` in ol-sim, `mod ai_provider` in ol-server main,
//! reqwest dep, docs patches (CALL_INDEX residual, field_map env paths).

use std::path::Path;
use std::process::Command;

pub fn ai_provider_wired(lib: &str) -> bool {
    (lib.contains("parse_provider_response") && lib.contains("build_ai_request_body"))
        || lib.contains("mod ai_provider;")
}

fn patch_call_index(workspace: &Path) {
    let call = workspace.join("docs/port/CALL_INDEX.md");
    let Ok(raw) = std::fs::read_to_string(&call) else {
        return;
    };
    let crlf = raw.contains("\r\n");
    let mut t = raw.replace("\r\n", "\n").replace('\r', "\n");

    let section = r#"### AI-PROVIDER / S-AIP (`AIProvider.hx` → `ai_handler` pure + `ol-server/ai_provider` HTTP)

| Haxe | Rust | Notes |
|------|------|-------|
| `IsLLMActivated` | `is_llm_activated` / `LlmEnvConfig::is_activated` | key ≠ empty / Not Set |
| `callAi` request body | `build_ai_request_body` | system dialog prompt; max_tokens |
| `callAi` URL | `ai_messages_endpoint` | `{AiApiUrl}/v1/messages` |
| `callAi` headers | `ai_request_headers` | Bearer + x-api-key + anthropic-version |
| `callAi` model/url | `resolve_ai_model` / `resolve_ai_api_url` | MiniMax defaults |
| `callAi` HTTP | `ol-server::ai_provider::call_ai` / `call_ai_async` / `CallAiParams` | reqwest; 120s; inject for `chat_response_with` |
| `parseResponse` | `parse_provider_response` | content[] text; choices fallback; type=error |
| secrets | env `AI_API_KEY`/`XAI_API_KEY`/`AI_API_URL`/`AI_DEFAULT_MODEL`/`AI_MAX_TOKENS_FOR_CHAT` | SecretOmit; never server.toml |
| Residual | — | **AI-LLM-WIRE** speech→async SAY; multi-server twins **parked** |
"#;

    if let Some(start) = t.find("### AI-PROVIDER / S-AIP") {
        // Replace from section header to EOF or next ### at line start after start+4
        let rest = &t[start + 4..];
        let end = if let Some(rel) = rest.find("\n### ") {
            start + 4 + rel + 1
        } else {
            t.len()
        };
        t = format!("{}{}{}", &t[..start], section, &t[end..]);
    } else {
        t = format!("{}\n{}\n", t.trim_end(), section);
    }

    // Fix leftover residual lines
    t = t.replace(
        "| Residual | — | live HTTP POST; multi-server twins **parked** |",
        "| Residual | — | **AI-LLM-WIRE** speech→async SAY; multi-server twins **parked** |",
    );

    let out = if crlf { t.replace('\n', "\r\n") } else { t };
    let _ = std::fs::write(&call, out);
}

fn patch_field_map(workspace: &Path) {
    let p = workspace.join("crates/ol-config/src/field_map.rs");
    let Ok(raw) = std::fs::read_to_string(&p) else {
        return;
    };
    let crlf = raw.contains("\r\n");
    let mut t = raw.replace("\r\n", "\n").replace('\r', "\n");
    t = t.replace(
        "rust_path: \"omit (env / operator secret; never settings dump)\",\n        home: SettingsHome::SecretOmit,\n    },\n    FieldEntry {\n        haxe_name: \"AiApiUrl\",\n        rust_path: \"omit from settings dump (LLM optional)\",",
        "rust_path: \"env AI_API_KEY|XAI_API_KEY only (AI-PROVIDER; never server.toml)\",\n        home: SettingsHome::SecretOmit,\n    },\n    FieldEntry {\n        haxe_name: \"AiApiUrl\",\n        rust_path: \"env AI_API_URL (AI-PROVIDER; never server.toml dump)\",",
    );
    t = t.replace(
        "haxe_name: \"AiDefaultModel\",\n        rust_path: \"omit from settings dump (LLM optional)\",",
        "haxe_name: \"AiDefaultModel\",\n        rust_path: \"env AI_DEFAULT_MODEL (AI-PROVIDER)\",",
    );
    let out = if crlf { t.replace('\n', "\r\n") } else { t };
    let _ = std::fs::write(&p, out);
}

pub fn patch_ai_provider(src: &Path, workspace: &Path) -> bool {
    let lib_path = src.join("lib.rs");
    let Ok(raw) = std::fs::read_to_string(&lib_path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut t = raw.replace("\r\n", "\n").replace('\r', "\n");

    let provider_rs = src.join("ai_provider.rs");
    if !provider_rs.exists() {
        eprintln!("cargo:warning=AI-PROVIDER: src/ai_provider.rs missing");
        return false;
    }

    if !t.contains("mod ai_provider;") {
        let anchors = [
            (
                "mod ai_handler;\n// Haxe: Connection.close",
                "mod ai_handler;\n// Haxe: openlife.server.AIProvider pure re-export (AI-PROVIDER / S-AIP llm_http)\nmod ai_provider;\n// Haxe: Connection.close",
            ),
            (
                "mod ai_handler;\n",
                "mod ai_handler;\n// Haxe: openlife.server.AIProvider pure re-export (AI-PROVIDER / S-AIP llm_http)\nmod ai_provider;\n",
            ),
        ];
        for (old, new) in anchors {
            if t.contains(old) && !t.contains("mod ai_provider;") {
                t = t.replacen(old, new, 1);
                break;
            }
        }
    }

    let out = if crlf { t.replace('\n', "\r\n") } else { t };
    let lib_ok = std::fs::write(&lib_path, &out).is_ok();
    let lib_check = std::fs::read_to_string(&lib_path).unwrap_or_default();
    let pure_ok = lib_check.contains("parse_provider_response")
        && lib_check.contains("build_ai_request_body");
    let wired = lib_ok && (pure_ok || ai_provider_wired(&lib_check));

    let ol_server_src = workspace.join("crates/ol-server/src");
    if !ol_server_src.join("ai_provider.rs").exists() {
        println!("cargo:warning=AI-PROVIDER: ol-server/src/ai_provider.rs missing (HTTP)");
    }

    let main_path = ol_server_src.join("main.rs");
    if let Ok(main_raw) = std::fs::read_to_string(&main_path) {
        if !main_raw.contains("mod ai_provider") {
            let main_crlf = main_raw.contains("\r\n");
            let mut m = main_raw.replace("\r\n", "\n").replace('\r', "\n");
            if m.contains("mod ai_llm_env;") {
                m = m.replacen(
                    "mod ai_llm_env;",
                    "mod ai_llm_env;\n// Haxe: AIProvider.callAi HTTP MiniMax/Anthropic (AI-PROVIDER llm_http)\nmod ai_provider;",
                    1,
                );
            }
            let mout = if main_crlf {
                m.replace('\n', "\r\n")
            } else {
                m
            };
            let _ = std::fs::write(&main_path, mout);
        }
    }

    let cargo_toml = workspace.join("crates/ol-server/Cargo.toml");
    if let Ok(ct) = std::fs::read_to_string(&cargo_toml) {
        if !ct.contains("reqwest") {
            let mut c = ct;
            if !c.ends_with('\n') {
                c.push('\n');
            }
            c.push_str(
                "# AI-PROVIDER: MiniMax / Anthropic-compatible HTTP (secrets via env only)\n\
reqwest = { version = \"0.12\", default-features = false, features = [\"rustls-tls\"] }\n",
            );
            let _ = std::fs::write(&cargo_toml, c);
        }
    }

    patch_call_index(workspace);
    patch_field_map(workspace);

    let py = workspace.join("docs/port/_apply_ai_provider_docs.py");
    if py.exists() {
        let _ = Command::new("python")
            .arg(&py)
            .status()
            .or_else(|_| Command::new("python3").arg(&py).status());
    }

    let stamp = src.join(".ai_provider_llm_http_patched");
    let _ = std::fs::write(&stamp, b"ai-provider-llm-http-3-http-rs-patched\n");

    if !wired {
        println!("cargo:warning=AI-PROVIDER: pure symbols or mod wire incomplete");
    }
    wired
}
