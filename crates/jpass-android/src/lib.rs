use jpass_core::{AppSettings, EncryptedBlob};
use jpass_platform::{BackupService, ClipboardService, PlatformError, PlatformPaths, VaultStore};
use std::path::PathBuf;

pub struct AndroidStore;

impl VaultStore for AndroidStore {
    type Error = PlatformError;

    fn load_vault(&self) -> Result<Option<EncryptedBlob>, Self::Error> {
        Err(PlatformError::Storage(
            "Android storage not implemented yet; use Android app-private storage or Keystore-backed persistence"
                .to_string(),
        ))
    }

    fn save_vault(&self, _blob: &EncryptedBlob) -> Result<(), Self::Error> {
        Err(PlatformError::Storage(
            "Android storage not implemented yet; use Android app-private storage or Keystore-backed persistence"
                .to_string(),
        ))
    }

    fn load_settings(&self) -> Result<AppSettings, Self::Error> {
        Ok(AppSettings::default())
    }

    fn save_settings(&self, _settings: &AppSettings) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl ClipboardService for AndroidStore {
    type Error = PlatformError;

    fn copy_text(&self, _text: &str) -> Result<(), Self::Error> {
        Err(PlatformError::Clipboard(
            "Android clipboard integration requires the Android runtime binding".to_string(),
        ))
    }

    fn clear(&self) -> Result<(), Self::Error> {
        Err(PlatformError::Clipboard(
            "Android clipboard clear requires the Android runtime binding".to_string(),
        ))
    }
}

impl BackupService for AndroidStore {
    type Error = PlatformError;

    fn save_encrypted_backup(&self, _blob: &EncryptedBlob) -> Result<PathBuf, Self::Error> {
        Err(PlatformError::Storage(
            "Android backup destination requires the native file picker binding".to_string(),
        ))
    }

    fn load_encrypted_backup(&self, _path: &std::path::Path) -> Result<EncryptedBlob, Self::Error> {
        Err(PlatformError::Storage(
            "Android backup loading requires the native file picker binding".to_string(),
        ))
    }
}

impl PlatformPaths for AndroidStore {
    type Error = PlatformError;

    fn data_dir(&self) -> Result<PathBuf, Self::Error> {
        Ok(PathBuf::from("/data/data/jpass"))
    }

    fn config_dir(&self) -> Result<PathBuf, Self::Error> {
        Ok(PathBuf::from("/data/data/jpass"))
    }
}

pub fn android_store() -> AndroidStore {
    AndroidStore
}
