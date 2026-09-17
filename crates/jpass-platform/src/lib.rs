use jpass_core::{AppSettings, EncryptedBlob, SyncEnvelope};
use std::path::PathBuf;

pub trait VaultStore {
    type Error: std::fmt::Display;

    fn load_vault(&self) -> Result<Option<EncryptedBlob>, Self::Error>;
    fn save_vault(&self, blob: &EncryptedBlob) -> Result<(), Self::Error>;
    fn load_settings(&self) -> Result<AppSettings, Self::Error>;
    fn save_settings(&self, settings: &AppSettings) -> Result<(), Self::Error>;
}

pub trait BackupService {
    type Error: std::fmt::Display;

    fn save_encrypted_backup(&self, blob: &EncryptedBlob) -> Result<PathBuf, Self::Error>;
    fn load_encrypted_backup(&self, path: &std::path::Path) -> Result<EncryptedBlob, Self::Error>;
}

pub trait SyncTransport {
    type Error: std::fmt::Display;

    fn upload(&self, envelope: &SyncEnvelope) -> Result<(), Self::Error>;
    fn download(&self) -> Result<Option<SyncEnvelope>, Self::Error>;
}

pub trait ClipboardService {
    type Error: std::fmt::Display;

    fn copy_text(&self, text: &str) -> Result<(), Self::Error>;
    fn clear(&self) -> Result<(), Self::Error>;
}

pub trait PlatformPaths {
    type Error: std::fmt::Display;

    fn data_dir(&self) -> Result<PathBuf, Self::Error>;
    fn config_dir(&self) -> Result<PathBuf, Self::Error>;
}

#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    #[error("platform storage error: {0}")]
    Storage(String),
    #[error("platform clipboard error: {0}")]
    Clipboard(String),
    #[error("path resolution error: {0}")]
    Path(String),
}

pub trait PlatformServices: VaultStore + BackupService + ClipboardService + PlatformPaths {}
