//! Persistence for the encrypted vault blob.
//!
//! The storage backend is selected via the platform adapter so the app is no
//! longer tied directly to a desktop-only implementation.

use crate::platform::{PlatformAdapter, PlatformAdapterTrait};
pub use jpass_core::{AppSettings, EncryptedBlob};
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("storage backend error: {0}")]
    Backend(String),
}

pub fn load_settings() -> Result<AppSettings, StorageError> {
    let adapter = PlatformAdapter::current();
    adapter
        .load_settings()
        .map_err(|e| StorageError::Backend(e.to_string()))
}

pub fn save_settings(settings: &AppSettings) -> Result<(), StorageError> {
    let adapter = PlatformAdapter::current();
    adapter
        .save_settings(settings)
        .map_err(|e| StorageError::Backend(e.to_string()))
}

pub fn load_encrypted() -> Result<Option<EncryptedBlob>, StorageError> {
    let adapter = PlatformAdapter::current();
    adapter
        .load_vault()
        .map_err(|e| StorageError::Backend(e.to_string()))
}

pub fn save_encrypted(blob: &EncryptedBlob) -> Result<(), StorageError> {
    let adapter = PlatformAdapter::current();
    adapter
        .save_vault(blob)
        .map_err(|e| StorageError::Backend(e.to_string()))
}

pub fn save_encrypted_backup(blob: &EncryptedBlob) -> Result<PathBuf, StorageError> {
    let adapter = PlatformAdapter::current();
    adapter
        .save_encrypted_backup(blob)
        .map_err(|e| StorageError::Backend(e.to_string()))
}
