//! Persistence for the encrypted vault blob.
//!
//! The storage backend is selected via the platform adapter so the app is no
//! longer tied directly to a desktop-only implementation.

use crate::platform::{PlatformAdapter, PlatformAdapterTrait};
use jpass_core::SyncEnvelope;
pub use jpass_core::{AppSettings, EncryptedBlob};
use std::path::Path;
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

pub fn load_encrypted_for_vault(vault_id: Option<&str>) -> Result<Option<EncryptedBlob>, StorageError> {
    let adapter = PlatformAdapter::current();
    adapter
        .load_vault_for_id(vault_id)
        .map_err(|e| StorageError::Backend(e.to_string()))
}

pub fn save_encrypted(blob: &EncryptedBlob) -> Result<(), StorageError> {
    let adapter = PlatformAdapter::current();
    adapter
        .save_vault(blob)
        .map_err(|e| StorageError::Backend(e.to_string()))
}

pub fn save_encrypted_for_vault(vault_id: Option<&str>, blob: &EncryptedBlob) -> Result<(), StorageError> {
    let adapter = PlatformAdapter::current();
    adapter
        .save_vault_for_id(vault_id, blob)
        .map_err(|e| StorageError::Backend(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vault_storage_uses_per_vault_file_names() {
        assert_eq!(PlatformAdapter::current().vault_file_name(Some("primary")), "vault-primary.sqlite3");
        assert_eq!(PlatformAdapter::current().vault_file_name(None), "vault.sqlite3");
    }
}

pub fn save_encrypted_backup(blob: &EncryptedBlob) -> Result<PathBuf, StorageError> {
    let adapter = PlatformAdapter::current();
    adapter
        .save_encrypted_backup(blob)
        .map_err(|e| StorageError::Backend(e.to_string()))
}

pub fn choose_backup_file() -> Result<Option<PathBuf>, StorageError> {
    let adapter = PlatformAdapter::current();
    adapter
        .choose_backup_file()
        .map_err(|e| StorageError::Backend(e.to_string()))
}

pub fn load_encrypted_backup(path: &Path) -> Result<EncryptedBlob, StorageError> {
    let adapter = PlatformAdapter::current();
    adapter
        .load_backup_file(path)
        .map_err(|e| StorageError::Backend(e.to_string()))
}

pub fn upload_sync(folder: &Path, envelope: &SyncEnvelope) -> Result<(), StorageError> {
    let adapter = PlatformAdapter::current();
    adapter
        .upload_sync(folder, envelope)
        .map_err(|e| StorageError::Backend(e.to_string()))
}

pub fn download_sync(folder: &Path) -> Result<Option<SyncEnvelope>, StorageError> {
    let adapter = PlatformAdapter::current();
    adapter
        .download_sync(folder)
        .map_err(|e| StorageError::Backend(e.to_string()))
}

pub fn choose_sync_folder() -> Result<Option<PathBuf>, StorageError> {
    let adapter = PlatformAdapter::current();
    adapter
        .choose_sync_folder()
        .map_err(|e| StorageError::Backend(e.to_string()))
}
