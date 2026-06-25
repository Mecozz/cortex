//! Shared lightweight LLM helper for background extraction (facts, scores,
//! tasks, relationships, summaries).
//!
//! Uses the Anthropic API when an API key is configured; otherwise falls back to
//! the Claude Code **subscription** by spawning the `claude` binary. This is what
//! lets memory capture work the same on either auth — without an API key,
//! subscription users still get facts/scores/tasks saved.

use reqwest::Client;
use serde::Deserialize;
use std::process::Stdio;
use tokio::io::AsyncWriteExt;

const HAIKU: &str = "claude-haiku-4-5-20251001";

/// Run one extraction turn. Returns the model's raw text, or None on failure.
/// `system` is the instruction prompt, `user` the content to process.
pub async fn haiku(api_key: &str, system: &str, user: &str, max_tokens: u32) -> Option<String> {
    if user.trim().is_empty() {
        return None;
    }
    if !api_key.is_empty() {
        api_haiku(api_key, system, user, max_tokens).await
    } else {
        sub_haiku(system, user).await
    }
}

async fn api_haiku(api_key: &str, system: &str, user: &str, max_tokens: u32) -> Option<String> {
    let body = serde_json::json!({
        "model": HAIKU,
        "max_tokens": max_tokens,
        "system": system,
        "messages": [{"role": "user", "content": user}]
    });
    let resp = Client::new()
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&body)
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    #[derive(Deserialize)]
    struct Block {
        text: String,
    }
    #[derive(Deserialize)]
    struct Resp {
        content: Vec<Block>,
    }
    let parsed: Resp = resp.json().await.ok()?;
    Some(parsed.content.into_iter().map(|c| c.text).collect())
}

/// Subscription fallback: spawn `claude` (read-only, sandboxed) with the system
/// prompt appended and the content on stdin. The extraction prompts are short,
/// so passing the system as a CLI arg is within the command-line limit.
async fn sub_haiku(system: &str, user: &str) -> Option<String> {
    let sandbox = std::env::temp_dir().join("cortex-sandbox");
    let _ = std::fs::create_dir_all(&sandbox);
    // REPLACE Claude Code's agent persona (via --system-prompt, not --append) so
    // it behaves as a plain extractor. With the default persona it answers
    // conversationally and tries to use tools instead of returning JSON.
    let sys = format!(
        "{system}\n\nYou are a JSON extraction engine. Output ONLY the JSON described \
         above — no commentary, no code fences, no tool use, no actions."
    );
    let args = [
        "--print",
        "--permission-mode",
        "plan",
        "--output-format",
        "text",
        "--model",
        HAIKU,
        "--system-prompt",
        sys.as_str(),
    ];

    #[cfg(target_os = "windows")]
    let mut child = {
        let mut full: Vec<&str> = vec!["/c", "claude"];
        full.extend_from_slice(&args);
        tokio::process::Command::new("cmd")
            .current_dir(&sandbox)
            .args(&full)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .ok()?
    };
    #[cfg(not(target_os = "windows"))]
    let mut child = tokio::process::Command::new("claude")
        .current_dir(&sandbox)
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(user.as_bytes()).await;
    }
    let out = child.wait_with_output().await.ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Trim model output to its first JSON array/object, so parsing survives the
/// subscription model occasionally wrapping JSON in prose or code fences.
pub fn json_slice(s: &str) -> &str {
    let t = s
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    match (t.find(['[', '{']), t.rfind([']', '}'])) {
        (Some(a), Some(b)) if b >= a => &t[a..=b],
        _ => t,
    }
}
