pub mod health;

use aes_gcm::{
    aead::{rand_core::RngCore, Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use rusqlite::{Connection, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;

pub struct VaultKey(pub [u8; 32]);

impl VaultKey {
    pub fn load_or_create(data_dir: &Path) -> std::io::Result<Self> {
        let key_path = data_dir.join("vault.key");
        if let Ok(bytes) = std::fs::read(&key_path) {
            if bytes.len() == 32 {
                let mut key = [0u8; 32];
                key.copy_from_slice(&bytes);
                return Ok(Self(key));
            }
        }
        let mut key = [0u8; 32];
        OsRng.fill_bytes(&mut key);
        std::fs::write(&key_path, &key)?;
        Ok(Self(key))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultEntry {
    pub key: String,
    pub description: String,
}

pub fn set(
    conn: &Connection,
    vault_key: &VaultKey,
    key: &str,
    value: &str,
    desc: Option<&str>,
) -> Result<()> {
    let encrypted = encrypt_value(vault_key, value);
    let now = now_secs();
    conn.execute(
        "INSERT INTO vault (id, key, value_encrypted, description, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?5)
         ON CONFLICT(key) DO UPDATE SET value_encrypted = excluded.value_encrypted,
         description = COALESCE(excluded.description, vault.description),
         updated_at = excluded.updated_at",
        rusqlite::params![Uuid::new_v4().to_string(), key, encrypted, desc, now],
    )?;
    Ok(())
}

pub fn get(conn: &Connection, vault_key: &VaultKey, key: &str) -> Option<String> {
    conn.query_row(
        "SELECT value_encrypted FROM vault WHERE key = ?1",
        rusqlite::params![key],
        |row| row.get::<_, String>(0),
    )
    .ok()
    .and_then(|enc| decrypt_value(vault_key, &enc))
}

pub fn list(conn: &Connection) -> Result<Vec<VaultEntry>> {
    let mut stmt = conn.prepare("SELECT key, COALESCE(description, '') FROM vault ORDER BY key")?;
    let rows = stmt.query_map([], |row| {
        Ok(VaultEntry {
            key: row.get(0)?,
            description: row.get(1)?,
        })
    })?;
    rows.collect()
}

pub fn delete(conn: &Connection, key: &str) -> Result<()> {
    conn.execute("DELETE FROM vault WHERE key = ?1", rusqlite::params![key])?;
    Ok(())
}

fn encrypt_value(vault_key: &VaultKey, plaintext: &str) -> String {
    let key = Key::<Aes256Gcm>::from_slice(&vault_key.0);
    let cipher = Aes256Gcm::new(key);
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .unwrap_or_default();
    let mut combined = nonce.to_vec();
    combined.extend(ciphertext);
    STANDARD.encode(&combined)
}

fn decrypt_value(vault_key: &VaultKey, encoded: &str) -> Option<String> {
    let bytes = STANDARD.decode(encoded).ok()?;
    if bytes.len() < 12 {
        return None;
    }
    let (nonce_bytes, ciphertext) = bytes.split_at(12);
    let key = Key::<Aes256Gcm>::from_slice(&vault_key.0);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(nonce_bytes);
    let plaintext = cipher.decrypt(nonce, ciphertext).ok()?;
    String::from_utf8(plaintext).ok()
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
