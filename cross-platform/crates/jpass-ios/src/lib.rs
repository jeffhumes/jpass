use jpass_core::{AppSettings, EncryptedBlob};
use jpass_platform::{ClipboardService, PlatformError, PlatformPaths, VaultStore};
use std::path::PathBuf;

pub struct IosStore;

impl VaultStore for IosStore {
    type Error = PlatformError;

    fn load_vault(&self) -> Result<Option<EncryptedBlob>, Self::Error> {
        Err(PlatformError::Storage(
            "iOS storage not implemented yet".to_string(),
        ))
    }

    fn save_vault(&self, _blob: &EncryptedBlob) -> Result<(), Self::Error> {
        Err(PlatformError::Storage(
            "iOS storage not implemented yet".to_string(),
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
            "iOS clipboard not implemented yet".to_string(),
        ))
    }

    fn clear(&self) -> Result<(), Self::Error> {
        Err(PlatformError::Clipboard(
            "iOS clipboard clear not implemented yet".to_string(),
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
