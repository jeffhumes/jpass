//! Persistence for the encrypted vault blob.
//!
//! - Desktop: a single-row SQLite database stored in the user's app-data
//!   directory. SQLite only stores the already-encrypted blob; no plaintext
//!   ever touches disk.
//! - Web: the same encrypted blob, stored under one key in the browser's
//!   `localStorage`.

use crate::crypto::EncryptedBlob;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppSettings {
    pub clipboard_timeout_secs: u64,
}

impl AppSettings {
    pub fn default_timeout() -> u64 {
        10
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("storage backend error: {0}")]
    Backend(String),
}

#[cfg(feature = "desktop")]
mod native {
    use super::*;
    use rusqlite::Connection;
    use std::path::PathBuf;

    fn db_path() -> Result<PathBuf, StorageError> {
        let dirs = directories::ProjectDirs::from("dev", "jpass", "JPass")
            .ok_or_else(|| StorageError::Backend("could not resolve app data directory".into()))?;
        let dir = dirs.data_dir();
        std::fs::create_dir_all(dir)
            .map_err(|e| StorageError::Backend(format!("failed to create data dir: {e}")))?;
        Ok(dir.join("vault.sqlite3"))
    }

    fn connection() -> Result<Connection, StorageError> {
        let conn = Connection::open(db_path()?)
            .map_err(|e| StorageError::Backend(format!("failed to open sqlite db: {e}")))?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS vault_blob (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                salt TEXT NOT NULL,
                nonce TEXT NOT NULL,
                ciphertext TEXT NOT NULL
            )",
            [],
        )
        .map_err(|e| StorageError::Backend(format!("failed to init schema: {e}")))?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS app_settings (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                clipboard_timeout_secs INTEGER NOT NULL
            )",
            [],
        )
        .map_err(|e| StorageError::Backend(format!("failed to init settings schema: {e}")))?;
        Ok(conn)
    }

    pub fn load_settings() -> Result<AppSettings, StorageError> {
        let conn = connection()?;
        let mut stmt = conn
            .prepare("SELECT clipboard_timeout_secs FROM app_settings WHERE id = 1")
            .map_err(|e| StorageError::Backend(e.to_string()))?;
        let mut rows = stmt
            .query_map([], |row| {
                Ok(AppSettings {
                    clipboard_timeout_secs: row.get(0)?,
                })
            })
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        match rows.next() {
            Some(row) => row.map_err(|e| StorageError::Backend(e.to_string())),
            None => Ok(AppSettings {
                clipboard_timeout_secs: AppSettings::default_timeout(),
            }),
        }
    }

    pub fn save_settings(settings: &AppSettings) -> Result<(), StorageError> {
        let conn = connection()?;
        conn.execute(
            "INSERT INTO app_settings (id, clipboard_timeout_secs) VALUES (1, ?1)
             ON CONFLICT(id) DO UPDATE SET clipboard_timeout_secs = excluded.clipboard_timeout_secs",
            (&settings.clipboard_timeout_secs,),
        )
        .map_err(|e| StorageError::Backend(format!("failed to save settings: {e}")))?;
        Ok(())
    }

    pub fn load_encrypted() -> Result<Option<EncryptedBlob>, StorageError> {
        let conn = connection()?;
        let mut stmt = conn
            .prepare("SELECT salt, nonce, ciphertext FROM vault_blob WHERE id = 1")
            .map_err(|e| StorageError::Backend(e.to_string()))?;
        let mut rows = stmt
            .query_map([], |row| {
                Ok(EncryptedBlob {
                    salt: row.get(0)?,
                    nonce: row.get(1)?,
                    ciphertext: row.get(2)?,
                })
            })
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        match rows.next() {
            Some(row) => Ok(Some(row.map_err(|e| StorageError::Backend(e.to_string()))?)),
            None => Ok(None),
        }
    }

    pub fn save_encrypted(blob: &EncryptedBlob) -> Result<(), StorageError> {
        let conn = connection()?;
        conn.execute(
            "INSERT INTO vault_blob (id, salt, nonce, ciphertext) VALUES (1, ?1, ?2, ?3)
             ON CONFLICT(id) DO UPDATE SET salt = excluded.salt, nonce = excluded.nonce, ciphertext = excluded.ciphertext",
            (&blob.salt, &blob.nonce, &blob.ciphertext),
        )
        .map_err(|e| StorageError::Backend(format!("failed to save vault: {e}")))?;
        Ok(())
    }
}

#[cfg(feature = "web")]
mod web {
    use super::*;
    use gloo_storage::{LocalStorage, Storage};

    const VAULT_KEY: &str = "jpass_vault_blob";
    const SETTINGS_KEY: &str = "jpass_app_settings";

    pub fn load_settings() -> Result<AppSettings, StorageError> {
        match LocalStorage::get::<AppSettings>(SETTINGS_KEY) {
            Ok(settings) => Ok(settings),
            Err(gloo_storage::errors::StorageError::KeyNotFound(_)) => Ok(AppSettings {
                clipboard_timeout_secs: AppSettings::default_timeout(),
            }),
            Err(e) => Err(StorageError::Backend(e.to_string())),
        }
    }

    pub fn save_settings(settings: &AppSettings) -> Result<(), StorageError> {
        LocalStorage::set(SETTINGS_KEY, settings).map_err(|e| StorageError::Backend(e.to_string()))
    }

    pub fn load_encrypted() -> Result<Option<EncryptedBlob>, StorageError> {
        match LocalStorage::get::<EncryptedBlob>(VAULT_KEY) {
            Ok(blob) => Ok(Some(blob)),
            Err(gloo_storage::errors::StorageError::KeyNotFound(_)) => Ok(None),
            Err(e) => Err(StorageError::Backend(e.to_string())),
        }
    }

    pub fn save_encrypted(blob: &EncryptedBlob) -> Result<(), StorageError> {
        LocalStorage::set(VAULT_KEY, blob).map_err(|e| StorageError::Backend(e.to_string()))
    }
}

#[cfg(feature = "desktop")]
pub use native::{load_encrypted, load_settings, save_encrypted, save_settings};

#[cfg(all(feature = "web", not(feature = "desktop")))]
pub use web::{load_encrypted, load_settings, save_encrypted, save_settings};
