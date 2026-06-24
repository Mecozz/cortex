pub mod health;

use reqwest::Client;

pub async fn notify(message: &str, bot_token: &str, chat_id: &str) -> Result<(), String> {
    if bot_token.is_empty() || chat_id.is_empty() {
        return Err(
            "Telegram not configured. Add telegram_bot_token and telegram_chat_id to Vault.".into(),
        );
    }
    let url = format!("https://api.telegram.org/bot{bot_token}/sendMessage");
    let body = serde_json::json!({
        "chat_id": chat_id,
        "text": message
    });
    let resp = Client::new()
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if resp.status().is_success() {
        Ok(())
    } else {
        Err(format!("Telegram API error: {}", resp.status()))
    }
}
