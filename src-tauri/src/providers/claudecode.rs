use tokio::io::AsyncWriteExt;

use crate::core::types::{CompletionRequest, CompletionResponse, ProviderError};

/// An empty, project-free directory to launch `claude` from in "chat" mode.
///
/// Claude Code auto-loads the working directory's files and any CLAUDE.md, so
/// spawning it in the app's own folder would leak project context. Chat mode
/// runs here so the subscription provider is a pure chat brain.
fn sandbox_dir() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join("cortex-sandbox");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn home_dir() -> std::path::PathBuf {
    std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir())
}

/// Resolve the permission args + working directory for the chosen access mode.
///
/// - "full": `--dangerously-skip-permissions` in a real directory (default:
///   the user's home) — a full agent that can read/edit files and run commands.
/// - anything else ("chat"): read-only `--permission-mode plan` in an empty
///   sandbox — cannot touch the filesystem.
///
/// SAFETY: "full" hands the model complete control of the machine. It is an
/// explicit, off-by-default opt-in (see Settings) so a fresh install is safe.
fn access_args(access: &str, workdir: &str) -> (Vec<String>, std::path::PathBuf) {
    if access == "full" {
        let dir = if workdir.trim().is_empty() {
            home_dir()
        } else {
            std::path::PathBuf::from(workdir)
        };
        (vec!["--dangerously-skip-permissions".to_string()], dir)
    } else {
        (
            vec!["--permission-mode".to_string(), "plan".to_string()],
            sandbox_dir(),
        )
    }
}

pub async fn complete(
    request: CompletionRequest,
    flag: String,
    session_id: String,
    access: String,
    workdir: String,
    abort: &std::sync::Mutex<Option<u32>>,
) -> Result<CompletionResponse, ProviderError> {
    let prompt = request
        .messages
        .iter()
        .rev()
        .find(|m| m.role == "user")
        .map(|m| m.content.clone())
        .unwrap_or_default();

    // Permission mode + working directory depend on the access setting:
    // "full" = --dangerously-skip-permissions in a real dir (full agent),
    // "chat" = --permission-mode plan in an empty sandbox (read-only brain).
    // (NOT `--bare`, which would disable OAuth and demand an API key —
    // incompatible with subscription auth.)
    let (perm_args, cwd) = access_args(&access, &workdir);

    // Build the claude arg list dynamically (perm_args length varies).
    let mut claude_args: Vec<String> = vec!["--print".to_string()];
    claude_args.extend(perm_args);
    claude_args.extend([
        "--output-format".to_string(),
        "text".to_string(),
        "--model".to_string(),
        request.model.clone(),
        flag,
        session_id,
    ]);

    // Windows: spawn via cmd /c so the claude.cmd wrapper resolves correctly.
    // Unix: call claude directly.
    #[cfg(target_os = "windows")]
    let mut child = {
        let mut full: Vec<String> = vec!["/c".to_string(), "claude".to_string()];
        full.extend(claude_args);
        tokio::process::Command::new("cmd")
            .current_dir(&cwd)
            .args(&full)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| ProviderError::NetworkError(format!("spawn: {}", e)))?
    };

    #[cfg(not(target_os = "windows"))]
    let mut child = {
        tokio::process::Command::new("claude")
            .current_dir(&cwd)
            .args(&claude_args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| ProviderError::NetworkError(format!("spawn: {}", e)))?
    };

    // Register the PID so `stop_chat` can kill this run mid-flight.
    if let Some(pid) = child.id() {
        if let Ok(mut g) = abort.lock() {
            *g = Some(pid);
        }
    }

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(prompt.as_bytes()).await;
    }

    let output = child
        .wait_with_output()
        .await
        .map_err(|e| ProviderError::NetworkError(e.to_string()))?;

    // Run finished (or was killed) — clear the active PID.
    if let Ok(mut g) = abort.lock() {
        *g = None;
    }

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let body = if !stderr.is_empty() { stderr } else { stdout };
        return Err(ProviderError::ApiError { status: 500, body });
    }

    Ok(CompletionResponse {
        content: String::from_utf8_lossy(&output.stdout).trim().to_string(),
        input_tokens: 0,
        output_tokens: 0,
        model: request.model,
        provider: "claude".into(),
    })
}
pub async fn complete_with_session(
    request: CompletionRequest,
    slot: &std::sync::Mutex<Option<String>>,
    access: &str,
    workdir: &str,
    abort: &std::sync::Mutex<Option<u32>>,
) -> Result<CompletionResponse, ProviderError> {
    let (flag, sid) = {
        let mut g = slot
            .lock()
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;
        if let Some(ref s) = *g {
            ("--resume".into(), s.clone())
        } else {
            let id = uuid::Uuid::new_v4().to_string();
            *g = Some(id.clone());
            ("--session-id".into(), id)
        }
    };
    complete(
        request,
        flag,
        sid,
        access.to_string(),
        workdir.to_string(),
        abort,
    )
    .await
}
