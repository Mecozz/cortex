use tokio::io::AsyncWriteExt;

use crate::core::types::{CompletionRequest, CompletionResponse, ProviderError};

pub async fn complete(
    request: CompletionRequest,
    flag: String,
    session_id: String,
) -> Result<CompletionResponse, ProviderError> {
    let prompt = request
        .messages
        .iter()
        .rev()
        .find(|m| m.role == "user")
        .map(|m| m.content.clone())
        .unwrap_or_default();

    // Windows: spawn via cmd /c so claude.cmd wrapper resolves correctly.
    // Unix: call claude directly.
    #[cfg(target_os = "windows")]
    let mut child = {
        tokio::process::Command::new("cmd")
            .args([
                "/c",
                "claude",
                "--print",
                "--dangerously-skip-permissions",
                "--output-format",
                "text",
                "--model",
                &request.model,
                &flag,
                &session_id,
            ])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| ProviderError::NetworkError(format!("spawn: {}", e)))?
    };

    #[cfg(not(target_os = "windows"))]
    let mut child = {
        tokio::process::Command::new("claude")
            .args([
                "--print",
                "--dangerously-skip-permissions",
                "--output-format",
                "text",
                "--model",
                &request.model,
                &flag,
                &session_id,
            ])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| ProviderError::NetworkError(format!("spawn: {}", e)))?
    };

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(prompt.as_bytes()).await;
    }

    let output = child
        .wait_with_output()
        .await
        .map_err(|e| ProviderError::NetworkError(e.to_string()))?;

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
    complete(request, flag, sid).await
}
