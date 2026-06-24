-- Settings: key-value store for app configuration (API keys, model selection, etc.)
-- API keys stored here until VAULT (Phase 5) replaces them.
CREATE TABLE IF NOT EXISTS settings (
    key         TEXT PRIMARY KEY NOT NULL,
    value       TEXT NOT NULL,
    updated_at  INTEGER NOT NULL DEFAULT (unixepoch())
);

-- Usage log: every API call tracked for cost visibility
CREATE TABLE IF NOT EXISTS usage (
    id            TEXT PRIMARY KEY NOT NULL,
    provider      TEXT NOT NULL,
    model         TEXT NOT NULL,
    input_tokens  INTEGER NOT NULL DEFAULT 0,
    output_tokens INTEGER NOT NULL DEFAULT 0,
    cost_usd      REAL NOT NULL DEFAULT 0.0,
    timestamp     INTEGER NOT NULL DEFAULT (unixepoch())
);

CREATE INDEX IF NOT EXISTS idx_usage_timestamp ON usage (timestamp);
