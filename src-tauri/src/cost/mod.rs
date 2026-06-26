pub mod health;

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageEntry {
    pub provider: String,
    pub model: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cost_usd: f64,
}

/// Approximate costs per 1M tokens (input / output) in USD.
/// Updated manually — not auto-fetched.
fn cost_per_mtok(_provider: &str, model: &str) -> (f64, f64) {
    if model.contains("haiku") {
        (1.00, 5.00) // Haiku 4.5
    } else if model.contains("sonnet") {
        (3.00, 15.00)
    } else if model.contains("opus") {
        (15.00, 75.00)
    } else if model.contains("claude") {
        (3.00, 15.00) // unknown Claude model → sonnet-class default
    } else {
        (0.0, 0.0)
    }
}

pub fn estimate_cost(provider: &str, model: &str, input: u32, output: u32) -> f64 {
    let (in_rate, out_rate) = cost_per_mtok(provider, model);
    (input as f64 / 1_000_000.0) * in_rate + (output as f64 / 1_000_000.0) * out_rate
}

pub fn log_usage(conn: &Connection, entry: &UsageEntry) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO usage (id, provider, model, input_tokens, output_tokens, cost_usd)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            Uuid::new_v4().to_string(),
            entry.provider,
            entry.model,
            entry.input_tokens,
            entry.output_tokens,
            entry.cost_usd,
        ],
    )?;
    Ok(())
}
