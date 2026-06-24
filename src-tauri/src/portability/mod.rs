pub mod health;

use rusqlite::{types::ValueRef, Connection};
use serde_json::{Map, Value};
use std::{
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

fn rows_as_json(conn: &Connection, sql: &str) -> Vec<Value> {
    let mut stmt = match conn.prepare(sql) {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    let names: Vec<String> = stmt
        .column_names()
        .into_iter()
        .map(|s| s.to_string())
        .collect();
    stmt.query_map([], |row| {
        let mut obj = Map::new();
        for (i, n) in names.iter().enumerate() {
            let v = match row.get_ref(i).unwrap_or(ValueRef::Null) {
                ValueRef::Null => Value::Null,
                ValueRef::Integer(x) => Value::Number(x.into()),
                ValueRef::Real(f) => serde_json::Number::from_f64(f)
                    .map(Value::Number)
                    .unwrap_or(Value::Null),
                ValueRef::Text(s) => Value::String(String::from_utf8_lossy(s).into()),
                ValueRef::Blob(_) => Value::Null,
            };
            obj.insert(n.clone(), v);
        }
        Ok(Value::Object(obj))
    })
    .map(|rows| rows.filter_map(|r| r.ok()).collect())
    .unwrap_or_default()
}

fn to_sql_val(v: &Value) -> rusqlite::types::Value {
    match v {
        Value::Null | Value::Array(_) | Value::Object(_) => rusqlite::types::Value::Null,
        Value::Bool(b) => rusqlite::types::Value::Integer(if *b { 1 } else { 0 }),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                rusqlite::types::Value::Integer(i)
            } else if let Some(f) = n.as_f64() {
                rusqlite::types::Value::Real(f)
            } else {
                rusqlite::types::Value::Null
            }
        }
        Value::String(s) => rusqlite::types::Value::Text(s.clone()),
    }
}

fn import_table(conn: &Connection, table: &str, rows: &[Value]) -> usize {
    let mut count = 0;
    for row in rows {
        let Some(obj) = row.as_object() else {
            continue;
        };
        let cols: Vec<&str> = obj.keys().map(|s| s.as_str()).collect();
        let ph: Vec<String> = (1..=cols.len()).map(|i| format!("?{i}")).collect();
        let sql = format!(
            "INSERT OR IGNORE INTO {table} ({}) VALUES ({})",
            cols.join(", "),
            ph.join(", ")
        );
        let vals: Vec<rusqlite::types::Value> = obj.values().map(to_sql_val).collect();
        if conn.execute(&sql, rusqlite::params_from_iter(vals)).is_ok() {
            count += 1;
        }
    }
    count
}

pub fn export_json(conn: &Connection, data_dir: &Path) -> Result<String, String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let data = serde_json::json!({
        "version": "1",
        "exported_at": now,
        "proj": rows_as_json(conn, "SELECT * FROM proj"),
        "facts": rows_as_json(conn, "SELECT * FROM facts"),
        "episodic": rows_as_json(conn, "SELECT * FROM episodic"),
        "tasks": rows_as_json(conn, "SELECT * FROM tasks"),
        "convo": rows_as_json(conn, "SELECT * FROM convo"),
        "rel": rows_as_json(conn, "SELECT * FROM rel"),
        "tools": rows_as_json(conn, "SELECT * FROM tools"),
    });
    let path = data_dir.join("cortex_export.json");
    let json = serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().into())
}

pub fn import_json(conn: &Connection, path: &str) -> Result<usize, String> {
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let data: Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    let tables = [
        "proj", "facts", "episodic", "tasks", "convo", "rel", "tools",
    ];
    let mut total = 0;
    for table in &tables {
        if let Some(rows) = data[table].as_array() {
            total += import_table(conn, table, rows);
        }
    }
    Ok(total)
}
