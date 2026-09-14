use jpass_core::{AppSettings, EncryptedBlob, SyncEnvelope};
use jpass_platform::{
    BackupService, ClipboardService, PlatformError, PlatformPaths, SyncTransport, VaultStore,
};
use std::fs;
use std::path::PathBuf;

pub struct DesktopStore;

pub struct LocalFolderSync {
    folder: PathBuf,
}

impl LocalFolderSync {
    pub fn new(folder: impl Into<PathBuf>) -> Result<Self, PlatformError> {
        let folder = folder.into();
        fs::create_dir_all(&folder)
            .map_err(|e| PlatformError::Storage(format!("failed to create sync folder: {e}")))?;
        Ok(Self { folder })
    }

    fn sync_path(&self) -> PathBuf {
        self.folder.join("jpass-sync.json")
    }
}

impl SyncTransport for LocalFolderSync {
    type Error = PlatformError;

    fn upload(&self, envelope: &SyncEnvelope) -> Result<(), Self::Error> {
        let bytes = serde_json::to_vec_pretty(envelope).map_err(|e| {
            PlatformError::Storage(format!("failed to serialize sync envelope: {e}"))
        })?;
        fs::write(self.sync_path(), bytes)
            .map_err(|e| PlatformError::Storage(format!("failed to write sync envelope: {e}")))
    }

    fn download(&self) -> Result<Option<SyncEnvelope>, Self::Error> {
        let path = self.sync_path();
        match fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map(Some)
                .map_err(|e| PlatformError::Storage(format!("failed to parse sync envelope: {e}"))),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(PlatformError::Storage(format!(
                "failed to read sync envelope: {error}"
            ))),
        }
    }
}

pub fn local_folder_sync(folder: impl Into<PathBuf>) -> Result<LocalFolderSync, PlatformError> {
    LocalFolderSync::new(folder)
}

impl VaultStore for DesktopStore {
    type Error = PlatformError;

    fn load_vault(&self) -> Result<Option<EncryptedBlob>, Self::Error> {
        let path = self
            .data_dir()
            .map_err(|e| PlatformError::Path(e.to_string()))?
            .join("vault.sqlite3");
        let conn =
            rusqlite::Connection::open(path).map_err(|e| PlatformError::Storage(e.to_string()))?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS vault_blob (id INTEGER PRIMARY KEY CHECK (id = 1), salt TEXT NOT NULL, nonce TEXT NOT NULL, ciphertext TEXT NOT NULL)",
            [],
        ).map_err(|e| PlatformError::Storage(e.to_string()))?;

        let mut stmt = conn
            .prepare("SELECT salt, nonce, ciphertext FROM vault_blob WHERE id = 1")
            .map_err(|e| PlatformError::Storage(e.to_string()))?;
        let mut rows = stmt
            .query_map([], |row| {
                Ok(EncryptedBlob {
                    salt: row.get(0)?,
                    nonce: row.get(1)?,
                    ciphertext: row.get(2)?,
                })
            })
            .map_err(|e| PlatformError::Storage(e.to_string()))?;

        match rows.next() {
            Some(row) => row
                .map(Some)
                .map_err(|e| PlatformError::Storage(e.to_string())),
            None => Ok(None),
        }
    }

    fn save_vault(&self, blob: &EncryptedBlob) -> Result<(), Self::Error> {
        let path = self
            .data_dir()
            .map_err(|e| PlatformError::Path(e.to_string()))?
            .join("vault.sqlite3");
        let conn =
            rusqlite::Connection::open(path).map_err(|e| PlatformError::Storage(e.to_string()))?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS vault_blob (id INTEGER PRIMARY KEY CHECK (id = 1), salt TEXT NOT NULL, nonce TEXT NOT NULL, ciphertext TEXT NOT NULL)",
            [],
        ).map_err(|e| PlatformError::Storage(e.to_string()))?;
        conn.execute(
            "INSERT INTO vault_blob (id, salt, nonce, ciphertext) VALUES (1, ?1, ?2, ?3) ON CONFLICT(id) DO UPDATE SET salt = excluded.salt, nonce = excluded.nonce, ciphertext = excluded.ciphertext",
            (&blob.salt, &blob.nonce, &blob.ciphertext),
        ).map_err(|e| PlatformError::Storage(e.to_string()))?;
        Ok(())
    }

    fn load_settings(&self) -> Result<AppSettings, Self::Error> {
        let path = self.settings_path()?;
        match fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|e| PlatformError::Storage(format!("failed to parse settings: {e}"))),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok(AppSettings::default())
            }
            Err(error) => Err(PlatformError::Storage(format!(
                "failed to read settings: {error}"
            ))),
        }
    }

    fn save_settings(&self, settings: &AppSettings) -> Result<(), Self::Error> {
        let path = self.settings_path()?;
        let bytes = serde_json::to_vec_pretty(settings)
            .map_err(|e| PlatformError::Storage(format!("failed to serialize settings: {e}")))?;
        fs::write(path, bytes)
            .map_err(|e| PlatformError::Storage(format!("failed to write settings: {e}")))?;
        Ok(())
    }
}

impl DesktopStore {
    fn settings_path(&self) -> Result<PathBuf, PlatformError> {
        Ok(self
            .config_dir()
            .map_err(|e| PlatformError::Path(e.to_string()))?
            .join("settings.json"))
    }
}

impl BackupService for DesktopStore {
    type Error = PlatformError;

    fn save_encrypted_backup(&self, blob: &EncryptedBlob) -> Result<PathBuf, Self::Error> {
        let backup_dir = self
            .data_dir()
            .map_err(|e| PlatformError::Path(e.to_string()))?
            .join("backups");
        fs::create_dir_all(&backup_dir).map_err(|e| {
            PlatformError::Storage(format!("failed to create backup directory: {e}"))
        })?;

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| {
                PlatformError::Storage(format!("failed to determine backup timestamp: {e}"))
            })?;
        let path = backup_dir.join(format!("jpass-backup-{}.json", timestamp.as_millis()));
        let bytes = serde_json::to_vec_pretty(blob)
            .map_err(|e| PlatformError::Storage(format!("failed to serialize backup: {e}")))?;
        fs::write(&path, bytes)
            .map_err(|e| PlatformError::Storage(format!("failed to write backup: {e}")))?;
        Ok(path)
    }
}

impl ClipboardService for DesktopStore {
    type Error = PlatformError;

    fn copy_text(&self, text: &str) -> Result<(), Self::Error> {
        let mut clipboard =
            arboard::Clipboard::new().map_err(|e| PlatformError::Clipboard(e.to_string()))?;
        clipboard
            .set_text(text.to_owned())
            .map_err(|e| PlatformError::Clipboard(e.to_string()))?;
        Ok(())
    }

    fn clear(&self) -> Result<(), Self::Error> {
        let mut clipboard =
            arboard::Clipboard::new().map_err(|e| PlatformError::Clipboard(e.to_string()))?;
        clipboard
            .clear()
            .map_err(|e| PlatformError::Clipboard(e.to_string()))?;
        Ok(())
    }
}

impl PlatformPaths for DesktopStore {
    type Error = PlatformError;

    fn data_dir(&self) -> Result<PathBuf, Self::Error> {
        let dir = directories::ProjectDirs::from("dev", "jpass", "JPass")
            .ok_or_else(|| PlatformError::Path("could not resolve app data directory".to_string()))?
            .data_dir()
            .to_path_buf();
        std::fs::create_dir_all(&dir)
            .map_err(|e| PlatformError::Path(format!("failed to create data dir: {e}")))?;
        Ok(dir)
    }

    fn config_dir(&self) -> Result<PathBuf, Self::Error> {
        let dir = directories::ProjectDirs::from("dev", "jpass", "JPass")
            .ok_or_else(|| PlatformError::Path("could not resolve config directory".to_string()))?
            .config_dir()
            .to_path_buf();
        std::fs::create_dir_all(&dir)
            .map_err(|e| PlatformError::Path(format!("failed to create config dir: {e}")))?;
        Ok(dir)
    }
}

pub fn desktop_store() -> DesktopStore {
    DesktopStore
}

#[cfg(test)]
mod sync_tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_folder() -> PathBuf {
        std::env::temp_dir().join(format!(
            "jpass-sync-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn local_folder_sync_round_trip() {
        let folder = test_folder();
        let sync = LocalFolderSync::new(&folder).unwrap();
        let envelope = SyncEnvelope::new(
            "test-device",
            7,
            EncryptedBlob {
                salt: "salt".into(),
                nonce: "nonce".into(),
                ciphertext: "ciphertext".into(),
            },
        );

        sync.upload(&envelope).unwrap();
        assert_eq!(sync.download().unwrap(), Some(envelope));
        let _ = fs::remove_dir_all(folder);
    }

    #[test]
    fn local_folder_sync_reports_missing_envelope() {
        let folder = test_folder();
        let sync = LocalFolderSync::new(&folder).unwrap();

        assert_eq!(sync.download().unwrap(), None);
        let _ = fs::remove_dir_all(folder);
    }
}
