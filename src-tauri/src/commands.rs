use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::State;

use crate::{
    cost::{self, UsageEntry},
    inject::Injector,
    memory,
    providers::{
        cloud::ClaudeProvider, fallback::FallbackPolicy, local::OllamaProvider, Message, Provider,
    },
    tasks, vault, watch,
};

pub struct DbState(pub Mutex<Connection>);
pub struct WatchState(pub Mutex<watch::CircuitBreaker>);
pub struct DbPath(pub std::path::PathBuf);
pub struct VaultState(pub Mutex<vault::VaultKey>);

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
    pub privacy_mode: bool,
    pub local_only: bool,
    pub sync_folder: String,
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
            privacy_mode: false,
            local_only: false,
            sync_folder: String::new(),
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

fn load_settings(conn: &Connection) -> Settings {
    let d = Settings::default();
    Settings {
        api_key_anthropic: get_setting(conn, "api_key_anthropic", &d.api_key_anthropic),
        api_key_openai: get_setting(conn, "api_key_openai", &d.api_key_openai),
        provider: get_setting(conn, "provider", &d.provider),
        model: get_setting(conn, "model", &d.model),
        system_prompt: get_setting(conn, "system_prompt", &d.system_prompt),
        fallback_policy: get_setting(conn, "fallback_policy", &d.fallback_policy),
        ollama_url: get_setting(conn, "ollama_url", &d.ollama_url),
        privacy_mode: get_setting(conn, "privacy_mode", "false") == "true",
        local_only: get_setting(conn, "local_only", "false") == "true",
        sync_folder: get_setting(conn, "sync_folder", ""),
    }
}

#[tauri::command]
pub fn get_settings(state: State<DbState>) -> Result<Settings, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    Ok(load_settings(&conn))
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
    let pv = settings.privacy_mode.to_string();
    set_setting(&conn, "privacy_mode", &pv).map_err(|e| e.to_string())?;
    let lv = settings.local_only.to_string();
    set_setting(&conn, "local_only", &lv).map_err(|e| e.to_string())?;
    set_setting(&conn, "sync_folder", &settings.sync_folder).map_err(|e| e.to_string())?;
    Ok(())
}

// ── Chat ──────────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn chat_message(
    messages: Vec<Message>,
    state: State<'_, DbState>,
    watch: State<'_, WatchState>,
    vault_state: State<'_, VaultState>,
) -> Result<crate::providers::CompletionResponse, String> {
    let (settings, known_facts, open_tasks) = match state.0.lock() {
        Ok(conn) => {
            let mut s = load_settings(&conn);
            // VAULT ➕ INJECT: use vault API key if settings key is empty
            if s.api_key_anthropic.is_empty() {
                if let Ok(vk) = vault_state.0.lock() {
                    if let Some(key) = vault::get(&conn, &vk, "api_key_anthropic") {
                        s.api_key_anthropic = key;
                    }
                }
            }
            let proj_id = memory::default_project_id(&conn).unwrap_or_default();
            let pass1 = memory::pass1::retrieve(&conn, &proj_id, 20).unwrap_or_default();
            let facts: Vec<String> = if pass1.is_empty() {
                memory::pass2::retrieve(&conn, &proj_id, 10)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|f| f.content)
                    .collect()
            } else {
                pass1.into_iter().map(|f| f.content).collect()
            };
            let task_list: Vec<String> = tasks::open(&conn, &proj_id, 10)
                .unwrap_or_default()
                .into_iter()
                .map(|t| t.content)
                .collect();
            (s, facts, task_list)
        }
        Err(_) => (Settings::default(), vec![], vec![]),
    };

    // LOCALONLY: block cloud providers when enabled
    if settings.local_only && settings.provider != "ollama" {
        return Err("Local-only mode is enabled. Switch provider to Ollama.".into());
    }

    // Circuit breaker: block disabled providers
    if watch
        .0
        .lock()
        .map(|cb| cb.is_disabled(&settings.provider))
        .unwrap_or(false)
    {
        return Err(format!(
            "Provider '{}' is temporarily disabled (circuit breaker).",
            settings.provider
        ));
    }

    let system = if settings.system_prompt.is_empty() {
        None
    } else {
        Some(settings.system_prompt.clone())
    };
    let request =
        Injector::new(system).assemble(messages, settings.model.clone(), &known_facts, &open_tasks);
    let policy: FallbackPolicy = settings.fallback_policy.parse().unwrap_or_default();

    let result = match settings.provider.as_str() {
        "ollama" => {
            OllamaProvider::new(settings.ollama_url.clone())
                .complete(request)
                .await
        }
        _ => {
            ClaudeProvider::new(settings.api_key_anthropic.clone())
                .complete(request)
                .await
        }
    };

    match result {
        Ok(resp) => {
            if let Ok(mut cb) = watch.0.lock() {
                cb.record_success(&settings.provider);
            }
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
        Err(e) => {
            if let Ok(mut cb) = watch.0.lock() {
                cb.record_failure(&settings.provider);
            }
            match policy {
                FallbackPolicy::Silent => Ok(crate::providers::CompletionResponse {
                    content: String::new(),
                    input_tokens: 0,
                    output_tokens: 0,
                    model: settings.model,
                    provider: settings.provider,
                }),
                _ => Err(e.to_string()),
            }
        }
    }
}

// ── Memory capture ────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn remember_turn(
    messages: Vec<Message>,
    conversation_id: String,
    state: State<'_, DbState>,
) -> Result<(), String> {
    let (api_key, proj_id, ep_count, privacy) = {
        let conn = state.0.lock().map_err(|e| e.to_string())?;
        let d = Settings::default();
        let key = get_setting(&conn, "api_key_anthropic", &d.api_key_anthropic);
        let priv_mode = get_setting(&conn, "privacy_mode", "false") == "true";
        if priv_mode {
            return Ok(());
        }
        let pid = memory::default_project_id(&conn).map_err(|e| e.to_string())?;
        for msg in &messages {
            let _ = memory::episodic::log(
                &conn,
                &pid,
                &memory::episodic::EpisodicEntry {
                    role: msg.role.clone(),
                    content: msg.content.clone(),
                    conversation_id: conversation_id.clone(),
                },
            );
        }
        let _ = memory::scores::flush_expired(&conn);
        let count = memory::episodic::count(&conn, &pid).unwrap_or(0);
        (key, pid, count, priv_mode)
    };
    let _ = privacy;

    let instcap_facts = memory::instcap::extract(&messages, &api_key).await;

    let rolled = if ep_count > 0 && ep_count % 6 == 0 {
        memory::rollext::score(&messages, &api_key).await
    } else {
        vec![]
    };

    let new_tasks = tasks::extract(&messages, &api_key).await;

    if let Ok(conn) = state.0.lock() {
        for fact in &instcap_facts {
            let _ = memory::conf::upsert(
                &conn,
                &memory::conf::Fact {
                    content: fact.content.clone(),
                    category: fact.category.clone(),
                    confidence: fact.confidence,
                    proj_id: proj_id.clone(),
                },
            );
        }
        for (content, score) in &rolled {
            if *score >= 7 {
                let _ = memory::conf::upsert(
                    &conn,
                    &memory::conf::Fact {
                        content: content.clone(),
                        category: "rolled".into(),
                        confidence: *score as f32 / 10.0,
                        proj_id: proj_id.clone(),
                    },
                );
            } else if *score >= 4 {
                let _ = memory::scores::buffer(&conn, content, *score as f64, &conversation_id);
            }
        }
        for task in &new_tasks {
            let _ = tasks::insert(&conn, &proj_id, task, &conversation_id);
        }
    }

    Ok(())
}

// ── Memory query ──────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct FactResult {
    pub content: String,
    pub category: String,
    pub confidence: f32,
}

#[tauri::command]
pub fn get_facts(state: State<DbState>) -> Result<Vec<FactResult>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let proj_id = memory::default_project_id(&conn).map_err(|e| e.to_string())?;
    let facts = memory::pass1::retrieve(&conn, &proj_id, 50).map_err(|e| e.to_string())?;
    Ok(facts
        .into_iter()
        .map(|f| FactResult {
            content: f.content,
            category: f.category,
            confidence: f.confidence,
        })
        .collect())
}

#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub role: String,
    pub content: String,
}

#[tauri::command]
pub fn search_memory(query: String, state: State<DbState>) -> Result<Vec<SearchResult>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let proj_id = memory::default_project_id(&conn).map_err(|e| e.to_string())?;
    let hits = memory::episodic::search(&conn, &proj_id, &query, 20).map_err(|e| e.to_string())?;
    Ok(hits
        .into_iter()
        .map(|(role, content)| SearchResult { role, content })
        .collect())
}

// ── Tasks ─────────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_tasks(state: State<DbState>) -> Result<Vec<tasks::Task>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let proj_id = memory::default_project_id(&conn).map_err(|e| e.to_string())?;
    tasks::open(&conn, &proj_id, 50).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn close_task(task_id: String, state: State<DbState>) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    tasks::close(&conn, &task_id).map_err(|e| e.to_string())
}

// ── Vault ─────────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_vault_keys(state: State<DbState>) -> Result<Vec<vault::VaultEntry>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    vault::list(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_vault_item(
    key: String,
    value: String,
    description: String,
    state: State<DbState>,
    vault_state: State<VaultState>,
) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let vk = vault_state.0.lock().map_err(|e| e.to_string())?;
    let desc = (!description.is_empty()).then_some(description.as_str());
    vault::set(&conn, &vk, &key, &value, desc).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_vault_item(key: String, state: State<DbState>) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    vault::delete(&conn, &key).map_err(|e| e.to_string())
}
