use jpass_core::{AppSettings, EncryptedBlob};
use jpass_platform::{BackupService, ClipboardService, VaultStore};
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
    fn save_encrypted_backup(&self, blob: &EncryptedBlob) -> Result<PathBuf, String>;
    fn load_settings(&self) -> Result<AppSettings, String>;
    fn save_settings(&self, settings: &AppSettings) -> Result<(), String>;
    fn copy_text(&self, text: &str) -> Result<(), String>;
    fn clear(&self) -> Result<(), String>;
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
}

impl PlatformAdapterTrait for PlatformAdapter {
    fn load_vault(&self) -> Result<Option<EncryptedBlob>, String> {
        match self.kind {
            #[cfg(feature = "desktop")]
            PlatformKind::Desktop => {
                let store = jpass_desktop::desktop_store();
                store.load_vault().map_err(|e| e.to_string())
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

    fn save_vault(&self, blob: &EncryptedBlob) -> Result<(), String> {
        match self.kind {
            #[cfg(feature = "desktop")]
            PlatformKind::Desktop => {
                let store = jpass_desktop::desktop_store();
                store.save_vault(blob).map_err(|e| e.to_string())
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
}
