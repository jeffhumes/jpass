use jpass_core::{AppSettings, EncryptedBlob};
use jpass_platform::{BackupService, ClipboardService, PlatformError, PlatformPaths, VaultStore};
use std::path::PathBuf;

pub struct IosStore;

impl VaultStore for IosStore {
    type Error = PlatformError;

    fn load_vault(&self) -> Result<Option<EncryptedBlob>, Self::Error> {
        Err(PlatformError::Storage(
            "iOS storage not implemented yet; use Keychain or app sandbox persistence".to_string(),
        ))
    }

    fn save_vault(&self, _blob: &EncryptedBlob) -> Result<(), Self::Error> {
        Err(PlatformError::Storage(
            "iOS storage not implemented yet; use Keychain or app sandbox persistence".to_string(),
        ))
    }

    fn load_settings(&self) -> Result<AppSettings, Self::Error> {
        Ok(AppSettings::default())
    }

    fn save_settings(&self, _settings: &AppSettings) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl ClipboardService for IosStore {
    type Error = PlatformError;

    fn copy_text(&self, _text: &str) -> Result<(), Self::Error> {
        Err(PlatformError::Clipboard(
            "iOS clipboard integration requires the iOS runtime binding".to_string(),
        ))
    }

    fn clear(&self) -> Result<(), Self::Error> {
        Err(PlatformError::Clipboard(
            "iOS clipboard clear requires the iOS runtime binding".to_string(),
        ))
    }
}

impl BackupService for IosStore {
    type Error = PlatformError;

    fn save_encrypted_backup(&self, _blob: &EncryptedBlob) -> Result<PathBuf, Self::Error> {
        Err(PlatformError::Storage(
            "iOS backup destination requires the native document picker binding".to_string(),
        ))
    }

    fn load_encrypted_backup(
        &self,
        _path: &std::path::Path,
    ) -> Result<EncryptedBlob, Self::Error> {
        Err(PlatformError::Storage(
            "iOS backup loading requires the native document picker binding".to_string(),
        ))
    }
}

impl PlatformPaths for IosStore {
    type Error = PlatformError;

    fn data_dir(&self) -> Result<PathBuf, Self::Error> {
        Ok(PathBuf::from("~/Library/Application Support/JPass"))
    }

    fn config_dir(&self) -> Result<PathBuf, Self::Error> {
        Ok(PathBuf::from("~/Library/Application Support/JPass"))
    }
}

pub fn ios_store() -> IosStore {
    IosStore
}
