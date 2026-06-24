use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::State;

use crate::{
    core::types::{CompletionResponse, Message, Provider, ProviderError},
    cost::{self, UsageEntry},
    inject::Injector,
    providers::{cloud::ClaudeProvider, fallback::FallbackPolicy, local::OllamaProvider},
};

pub struct DbState(pub Mutex<Connection>);

// ── Settings ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub api_key_anthropic: String,
    pub api_key_openai: String,
    pub provider: String,
    pub model: String,
    pub system_prompt: String,
    pub fallback_policy: String,
    pub ollama_url: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            api_key_anthropic: String::new(),
            api_key_openai: String::new(),
            provider: "claude".into(),
            model: "claude-sonnet-4-6".into(),
            system_prompt: String::new(),
            fallback_policy: "hard_fail".into(),
            ollama_url: "http://localhost:11434".into(),
        }
    }
}

fn get_setting(conn: &Connection, key: &str, default: &str) -> String {
    conn.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        params![key],
        |row| row.get::<_, String>(0),
    )
    .unwrap_or_else(|_| default.to_string())
}

fn set_setting(conn: &Connection, key: &str, value: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = unixepoch()",
        params![key, value],
    )?;
    Ok(())
}

#[tauri::command]
pub fn get_settings(state: State<DbState>) -> Result<Settings, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let d = Settings::default();
    Ok(Settings {
        api_key_anthropic: get_setting(&conn, "api_key_anthropic", &d.api_key_anthropic),
        api_key_openai: get_setting(&conn, "api_key_openai", &d.api_key_openai),
        provider: get_setting(&conn, "provider", &d.provider),
        model: get_setting(&conn, "model", &d.model),
        system_prompt: get_setting(&conn, "system_prompt", &d.system_prompt),
        fallback_policy: get_setting(&conn, "fallback_policy", &d.fallback_policy),
        ollama_url: get_setting(&conn, "ollama_url", &d.ollama_url),
    })
}

#[tauri::command]
pub fn save_settings(settings: Settings, state: State<DbState>) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    set_setting(&conn, "api_key_anthropic", &settings.api_key_anthropic)
        .map_err(|e| e.to_string())?;
    set_setting(&conn, "api_key_openai", &settings.api_key_openai).map_err(|e| e.to_string())?;
    set_setting(&conn, "provider", &settings.provider).map_err(|e| e.to_string())?;
    set_setting(&conn, "model", &settings.model).map_err(|e| e.to_string())?;
    set_setting(&conn, "system_prompt", &settings.system_prompt).map_err(|e| e.to_string())?;
    set_setting(&conn, "fallback_policy", &settings.fallback_policy).map_err(|e| e.to_string())?;
    set_setting(&conn, "ollama_url", &settings.ollama_url).map_err(|e| e.to_string())?;
    Ok(())
}

// ── Chat ──────────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn chat_message(
    messages: Vec<Message>,
    state: State<'_, DbState>,
) -> Result<CompletionResponse, String> {
    let settings = {
        let conn = state.0.lock().map_err(|e| e.to_string())?;
        let d = Settings::default();
        Settings {
            api_key_anthropic: get_setting(&conn, "api_key_anthropic", &d.api_key_anthropic),
            api_key_openai: get_setting(&conn, "api_key_openai", &d.api_key_openai),
            provider: get_setting(&conn, "provider", &d.provider),
            model: get_setting(&conn, "model", &d.model),
            system_prompt: get_setting(&conn, "system_prompt", &d.system_prompt),
            fallback_policy: get_setting(&conn, "fallback_policy", &d.fallback_policy),
            ollama_url: get_setting(&conn, "ollama_url", &d.ollama_url),
        }
    };

    let system = if settings.system_prompt.is_empty() {
        None
    } else {
        Some(settings.system_prompt.clone())
    };

    let injector = Injector::new(system);
    let request = injector.assemble(messages, settings.model.clone());

    let policy = FallbackPolicy::from_str(&settings.fallback_policy);

    let result: Result<CompletionResponse, ProviderError> = match settings.provider.as_str() {
        "ollama" => {
            let p = OllamaProvider::new(settings.ollama_url.clone());
            p.complete(request).await
        }
        _ => {
            let p = ClaudeProvider::new(settings.api_key_anthropic.clone());
            p.complete(request).await
        }
    };

    match result {
        Ok(resp) => {
            let entry = UsageEntry {
                provider: resp.provider.clone(),
                model: resp.model.clone(),
                input_tokens: resp.input_tokens,
                output_tokens: resp.output_tokens,
                cost_usd: cost::estimate_cost(
                    &resp.provider,
                    &resp.model,
                    resp.input_tokens,
                    resp.output_tokens,
                ),
            };
            if let Ok(conn) = state.0.lock() {
                let _ = cost::log_usage(&conn, &entry);
            }
            Ok(resp)
        }
        Err(e) => match policy {
            FallbackPolicy::Silent => Ok(CompletionResponse {
                content: String::new(),
                input_tokens: 0,
                output_tokens: 0,
                model: settings.model,
                provider: settings.provider,
            }),
            _ => Err(e.to_string()),
        },
    }
}
