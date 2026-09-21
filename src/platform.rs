use jpass_core::SyncEnvelope;
use jpass_core::{AppSettings, EncryptedBlob};
use jpass_platform::{BackupService, ClipboardService, SyncTransport, VaultStore};
use std::path::Path;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy)]
pub enum PlatformKind {
    Desktop,
    Android,
    Ios,
    Web,
}

pub trait PlatformAdapterTrait {
    fn load_vault(&self) -> Result<Option<EncryptedBlob>, String>;
    fn save_vault(&self, blob: &EncryptedBlob) -> Result<(), String>;
    fn load_vault_for_id(&self, vault_id: Option<&str>) -> Result<Option<EncryptedBlob>, String>;
    fn save_vault_for_id(&self, vault_id: Option<&str>, blob: &EncryptedBlob)
        -> Result<(), String>;
    fn vault_file_name(&self, vault_id: Option<&str>) -> String;
    fn save_encrypted_backup(&self, blob: &EncryptedBlob) -> Result<PathBuf, String>;
    fn choose_backup_file(&self) -> Result<Option<PathBuf>, String>;
    fn load_backup_file(&self, path: &Path) -> Result<EncryptedBlob, String>;
    fn load_settings(&self) -> Result<AppSettings, String>;
    fn save_settings(&self, settings: &AppSettings) -> Result<(), String>;
    fn delete_vault_for_id(&self, vault_id: Option<&str>) -> Result<(), String>;
    fn copy_text(&self, text: &str) -> Result<(), String>;
    fn clear(&self) -> Result<(), String>;
    fn upload_sync(&self, folder: &Path, envelope: &SyncEnvelope) -> Result<(), String>;
    fn download_sync(&self, folder: &Path) -> Result<Option<SyncEnvelope>, String>;
    fn choose_sync_folder(&self) -> Result<Option<PathBuf>, String>;
}

#[derive(Clone, Copy)]
pub struct PlatformAdapter {
    kind: PlatformKind,
}

impl PlatformAdapter {
    pub fn current() -> Self {
        Self {
            kind: PlatformKind::Desktop,
        }
    }

    pub fn kind(&self) -> PlatformKind {
        self.kind
    }

    pub fn vault_file_name(&self, vault_id: Option<&str>) -> String {
        match vault_id {
            Some(id) => {
                let sanitized: String = id
                    .chars()
                    .filter(|ch| ch.is_ascii_alphanumeric() || matches!(*ch, '-' | '_'))
                    .collect();
                if sanitized.is_empty() {
                    "vault.sqlite3".to_string()
                } else {
                    format!("vault-{sanitized}.sqlite3")
                }
            }
            None => "vault.sqlite3".to_string(),
        }
    }
}

impl PlatformAdapterTrait for PlatformAdapter {
    fn load_vault(&self) -> Result<Option<EncryptedBlob>, String> {
        PlatformAdapterTrait::load_vault_for_id(self, None)
    }

    fn save_vault(&self, blob: &EncryptedBlob) -> Result<(), String> {
        PlatformAdapterTrait::save_vault_for_id(self, None, blob)
    }

    fn load_vault_for_id(&self, vault_id: Option<&str>) -> Result<Option<EncryptedBlob>, String> {
        match self.kind {
            #[cfg(feature = "desktop")]
            PlatformKind::Desktop => {
                let store = jpass_desktop::desktop_store();
                store
                    .load_vault_for_id(vault_id)
                    .map_err(|e| e.to_string())
            }
            #[cfg(not(feature = "desktop"))]
            PlatformKind::Desktop => {
                Err("Desktop adapter is not enabled for this build".to_string())
            }
            PlatformKind::Android => {
                let store = jpass_android::android_store();
                store.load_vault().map_err(|e| e.to_string())
            }
            PlatformKind::Ios => {
                let store = jpass_ios::ios_store();
                store.load_vault().map_err(|e| e.to_string())
            }
            PlatformKind::Web => Err("Web adapter not implemented in this repo yet".to_string()),
        }
    }

    fn save_vault_for_id(
        &self,
        vault_id: Option<&str>,
        blob: &EncryptedBlob,
    ) -> Result<(), String> {
        match self.kind {
            #[cfg(feature = "desktop")]
            PlatformKind::Desktop => {
                let store = jpass_desktop::desktop_store();
                store
                    .save_vault_for_id(vault_id, blob)
                    .map_err(|e| e.to_string())
            }
            #[cfg(not(feature = "desktop"))]
            PlatformKind::Desktop => {
                Err("Desktop adapter is not enabled for this build".to_string())
            }
            PlatformKind::Android => {
                let store = jpass_android::android_store();
                store.save_vault(blob).map_err(|e| e.to_string())
            }
            PlatformKind::Ios => {
                let store = jpass_ios::ios_store();
                store.save_vault(blob).map_err(|e| e.to_string())
            }
            PlatformKind::Web => Err("Web adapter not implemented in this repo yet".to_string()),
        }
    }

    fn delete_vault_for_id(&self, vault_id: Option<&str>) -> Result<(), String> {
        match self.kind {
            #[cfg(feature = "desktop")]
            PlatformKind::Desktop => jpass_desktop::desktop_store()
                .delete_vault_for_id(vault_id)
                .map_err(|e| e.to_string()),
            #[cfg(not(feature = "desktop"))]
            PlatformKind::Desktop => {
                Err("Desktop adapter is not enabled for this build".to_string())
            }
            PlatformKind::Android => Err("Vault deletion is not implemented for Android".into()),
            PlatformKind::Ios => Err("Vault deletion is not implemented for iOS".into()),
            PlatformKind::Web => Err("Vault deletion is not implemented for web".into()),
        }
    }

    fn vault_file_name(&self, vault_id: Option<&str>) -> String {
        PlatformAdapter::vault_file_name(self, vault_id)
    }

    fn save_encrypted_backup(&self, blob: &EncryptedBlob) -> Result<PathBuf, String> {
        match self.kind {
            #[cfg(feature = "desktop")]
            PlatformKind::Desktop => jpass_desktop::desktop_store()
                .save_encrypted_backup(blob)
                .map_err(|e| e.to_string()),
            #[cfg(not(feature = "desktop"))]
            PlatformKind::Desktop => {
                Err("Desktop adapter is not enabled for this build".to_string())
            }
            PlatformKind::Android => jpass_android::android_store()
                .save_encrypted_backup(blob)
                .map_err(|e| e.to_string()),
            PlatformKind::Ios => jpass_ios::ios_store()
                .save_encrypted_backup(blob)
                .map_err(|e| e.to_string()),
            PlatformKind::Web => {
                Err("Web backup adapter not implemented in this repo yet".to_string())
            }
        }
    }

    fn choose_backup_file(&self) -> Result<Option<PathBuf>, String> {
        match self.kind {
            #[cfg(feature = "desktop")]
            PlatformKind::Desktop => Ok(jpass_desktop::choose_backup_file()),
            _ => Err("Backup file picker is not implemented for this platform".to_string()),
        }
    }

    fn load_backup_file(&self, path: &Path) -> Result<EncryptedBlob, String> {
        match self.kind {
            #[cfg(feature = "desktop")]
            PlatformKind::Desktop => jpass_desktop::desktop_store()
                .load_encrypted_backup(path)
                .map_err(|e| e.to_string()),
            #[cfg(not(feature = "desktop"))]
            PlatformKind::Desktop => {
                Err("Desktop adapter is not enabled for this build".to_string())
            }
            PlatformKind::Android => jpass_android::android_store()
                .load_encrypted_backup(path)
                .map_err(|e| e.to_string()),
            PlatformKind::Ios => jpass_ios::ios_store()
                .load_encrypted_backup(path)
                .map_err(|e| e.to_string()),
            PlatformKind::Web => {
                Err("Web backup loading is not implemented in this repo yet".to_string())
            }
        }
    }

    fn load_settings(&self) -> Result<AppSettings, String> {
        match self.kind {
            #[cfg(feature = "desktop")]
            PlatformKind::Desktop => {
                let store = jpass_desktop::desktop_store();
                store.load_settings().map_err(|e| e.to_string())
            }
            #[cfg(not(feature = "desktop"))]
            PlatformKind::Desktop => {
                Err("Desktop adapter is not enabled for this build".to_string())
            }
            PlatformKind::Android => {
                let store = jpass_android::android_store();
                store.load_settings().map_err(|e| e.to_string())
            }
            PlatformKind::Ios => {
                let store = jpass_ios::ios_store();
                store.load_settings().map_err(|e| e.to_string())
            }
            PlatformKind::Web => Err("Web adapter not implemented in this repo yet".to_string()),
        }
    }

    fn save_settings(&self, settings: &AppSettings) -> Result<(), String> {
        match self.kind {
            #[cfg(feature = "desktop")]
            PlatformKind::Desktop => {
                let store = jpass_desktop::desktop_store();
                store.save_settings(settings).map_err(|e| e.to_string())
            }
            #[cfg(not(feature = "desktop"))]
            PlatformKind::Desktop => {
                Err("Desktop adapter is not enabled for this build".to_string())
            }
            PlatformKind::Android => {
                let store = jpass_android::android_store();
                store.save_settings(settings).map_err(|e| e.to_string())
            }
            PlatformKind::Ios => {
                let store = jpass_ios::ios_store();
                store.save_settings(settings).map_err(|e| e.to_string())
            }
            PlatformKind::Web => Err("Web adapter not implemented in this repo yet".to_string()),
        }
    }

    fn copy_text(&self, text: &str) -> Result<(), String> {
        match self.kind {
            #[cfg(feature = "desktop")]
            PlatformKind::Desktop => {
                let store = jpass_desktop::desktop_store();
                store.copy_text(text).map_err(|e| e.to_string())
            }
            #[cfg(not(feature = "desktop"))]
            PlatformKind::Desktop => {
                Err("Desktop adapter is not enabled for this build".to_string())
            }
            PlatformKind::Android => {
                let store = jpass_android::android_store();
                store.copy_text(text).map_err(|e| e.to_string())
            }
            PlatformKind::Ios => {
                let store = jpass_ios::ios_store();
                store.copy_text(text).map_err(|e| e.to_string())
            }
            PlatformKind::Web => Err("Web adapter not implemented in this repo yet".to_string()),
        }
    }

    fn clear(&self) -> Result<(), String> {
        match self.kind {
            #[cfg(feature = "desktop")]
            PlatformKind::Desktop => {
                let store = jpass_desktop::desktop_store();
                store.clear().map_err(|e| e.to_string())
            }
            #[cfg(not(feature = "desktop"))]
            PlatformKind::Desktop => {
                Err("Desktop adapter is not enabled for this build".to_string())
            }
            PlatformKind::Android => {
                let store = jpass_android::android_store();
                store.clear().map_err(|e| e.to_string())
            }
            PlatformKind::Ios => {
                let store = jpass_ios::ios_store();
                store.clear().map_err(|e| e.to_string())
            }
            PlatformKind::Web => Err("Web adapter not implemented in this repo yet".to_string()),
        }
    }

    fn upload_sync(&self, folder: &Path, envelope: &SyncEnvelope) -> Result<(), String> {
        match self.kind {
            #[cfg(feature = "desktop")]
            PlatformKind::Desktop => jpass_desktop::local_folder_sync(folder)
                .map_err(|e| e.to_string())?
                .upload(envelope)
                .map_err(|e| e.to_string()),
            _ => Err("Sync transport is not implemented for this platform".to_string()),
        }
    }

    fn download_sync(&self, folder: &Path) -> Result<Option<SyncEnvelope>, String> {
        match self.kind {
            #[cfg(feature = "desktop")]
            PlatformKind::Desktop => jpass_desktop::local_folder_sync(folder)
                .map_err(|e| e.to_string())?
                .download()
                .map_err(|e| e.to_string()),
            _ => Err("Sync transport is not implemented for this platform".to_string()),
        }
    }

    fn choose_sync_folder(&self) -> Result<Option<PathBuf>, String> {
        match self.kind {
            #[cfg(feature = "desktop")]
            PlatformKind::Desktop => Ok(jpass_desktop::choose_sync_folder()),
            _ => Err("Folder picker is not implemented for this platform".to_string()),
        }
    }
}
