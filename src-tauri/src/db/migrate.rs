use rusqlite::Connection;
use std::time::{SystemTime, UNIX_EPOCH};

const MIGRATIONS: &[(&str, &str)] = &[
    (
        "001",
        include_str!("../../migrations/001_initial_schema.sql"),
    ),
    ("002", include_str!("../../migrations/002_settings.sql")),
];

pub fn run(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version     TEXT PRIMARY KEY,
            applied_at  INTEGER NOT NULL
        );",
    )?;

    for (version, sql) in MIGRATIONS {
        let exists: bool = conn.query_row(
            "SELECT COUNT(*) > 0 FROM schema_migrations WHERE version = ?1",
            [version],
            |row| row.get(0),
        )?;

        if !exists {
            conn.execute_batch(sql)?;
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            conn.execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
                rusqlite::params![version, now],
            )?;
        }
    }

    Ok(())
}
