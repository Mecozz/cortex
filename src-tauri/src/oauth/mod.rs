use base64::{engine::general_purpose, Engine};
use sha2::{Digest, Sha256};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

const CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";
const AUTHORIZE_URL: &str = "https://claude.com/cai/oauth/authorize";
const TOKEN_URL: &str = "https://platform.claude.com/v1/oauth/token";
const SCOPE: &str = "user:inference user:profile";

fn urlencode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{:02X}", b),
        })
        .collect()
}

fn generate_code_verifier() -> String {
    let u1 = uuid::Uuid::new_v4().simple().to_string();
    let u2 = uuid::Uuid::new_v4().simple().to_string();
    format!("{}{}", u1, u2)
}

fn code_challenge(verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    general_purpose::URL_SAFE_NO_PAD.encode(hasher.finalize())
}

fn extract_code(request: &str) -> Result<String, String> {
    let path = request
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .ok_or("Malformed HTTP request")?;
    path.split('?')
        .nth(1)
        .unwrap_or("")
        .split('&')
        .find_map(|kv| {
            let mut parts = kv.splitn(2, '=');
            if parts.next() == Some("code") {
                parts.next().map(String::from)
            } else {
                None
            }
        })
        .ok_or_else(|| "No authorization code in callback".to_string())
}

fn save_credentials(access: &str, refresh: &str, expires_in: u64) {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_default();
    let path = std::path::Path::new(&home)
        .join(".claude")
        .join(".credentials.json");
    let expires_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64 + expires_in * 1000)
        .unwrap_or(0);
    let creds = serde_json::json!({
        "claudeAiOauth": {
            "accessToken": access,
            "refreshToken": refresh,
            "expiresAt": expires_at
        }
    });
    let _ = std::fs::write(&path, serde_json::to_string_pretty(&creds).unwrap_or_default());
}

pub async fn begin_oauth(app: &tauri::AppHandle) -> Result<String, String> {
    use tauri_plugin_opener::OpenerExt;

    let verifier = generate_code_verifier();
    let challenge = code_challenge(&verifier);

    let redirect_uri = "http://127.0.0.1/callback".to_string();
    let listener = TcpListener::bind("127.0.0.1:80")
        .await
        .map_err(|e| format!("Cannot bind port 80: {}", e))?;

    let state = uuid::Uuid::new_v4().simple().to_string();
    let auth_url = format!(
        "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&code_challenge={}&code_challenge_method=S256&state={}",
        AUTHORIZE_URL,
        urlencode(CLIENT_ID),
        urlencode(&redirect_uri),
        urlencode(SCOPE),
        challenge,
        state
    );

    app.opener()
        .open_url(&auth_url, None::<&str>)
        .map_err(|e| e.to_string())?;

    let (mut stream, _) = listener.accept().await.map_err(|e| e.to_string())?;
    let mut buf = vec![0u8; 8192];
    let n = stream.read(&mut buf).await.map_err(|e| e.to_string())?;
    let request = String::from_utf8_lossy(&buf[..n]).to_string();

    let code = extract_code(&request)?;

    let html = b"<html><body><h2>Authenticated! You can close this tab.</h2></body></html>";
    let _ = stream
        .write_all(
            format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                html.len()
            )
            .as_bytes(),
        )
        .await;
    let _ = stream.write_all(html).await;

    let client = reqwest::Client::new();
    let token_redirect = "http://127.0.0.1/callback";
    let body = format!(
        "grant_type=authorization_code&client_id={}&code={}&redirect_uri={}&code_verifier={}",
        urlencode(CLIENT_ID),
        urlencode(&code),
        urlencode(token_redirect),
        urlencode(&verifier)
    );
    let resp = client
        .post(TOKEN_URL)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body.clone())
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let txt = resp.text().await.unwrap_or_default();
        return Err(format!("[{}] sent: {} | got: {}", status, body, txt));
    }

    let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let access = body["access_token"]
        .as_str()
        .ok_or("No access_token")?
        .to_string();
    let refresh = body["refresh_token"].as_str().unwrap_or("").to_string();
    let expires_in = body["expires_in"].as_u64().unwrap_or(28800);

    save_credentials(&access, &refresh, expires_in);

    Ok(access)
}
