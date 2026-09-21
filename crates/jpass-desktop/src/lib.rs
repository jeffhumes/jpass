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

pub fn choose_sync_folder() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_title("Choose JPass sync folder")
        .pick_folder()
}

pub fn choose_backup_file() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_title("Choose JPass backup file")
        .add_filter("JPass backup", &["json"])
        .pick_file()
}

impl VaultStore for DesktopStore {
    type Error = PlatformError;

    fn load_vault(&self) -> Result<Option<EncryptedBlob>, Self::Error> {
        self.load_vault_for_id(None)
    }

    fn save_vault(&self, blob: &EncryptedBlob) -> Result<(), Self::Error> {
        self.save_vault_for_id(None, blob)
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

    fn delete_vault_for_id(&self, vault_id: Option<&str>) -> Result<(), Self::Error> {
        DesktopStore::delete_vault_for_id(self, vault_id)
    }
}

impl DesktopStore {
    pub fn load_vault_for_id(
        &self,
        vault_id: Option<&str>,
    ) -> Result<Option<EncryptedBlob>, PlatformError> {
        let path = self
            .data_dir()
            .map_err(|e| PlatformError::Path(e.to_string()))?
            .join(vault_file_name(vault_id));
        let conn =
            rusqlite::Connection::open(path).map_err(|e| PlatformError::Storage(e.to_string()))?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS vault_blob (id INTEGER PRIMARY KEY CHECK (id = 1), salt TEXT NOT NULL, nonce TEXT NOT NULL, ciphertext TEXT NOT NULL)",
            [],
        )
        .map_err(|e| PlatformError::Storage(e.to_string()))?;

        let mut stmt = conn
            .prepare("SELECT salt, nonce, ciphertext FROM vault_blob WHERE id = 1")
            .map_err(|e| PlatformError::Storage(e.to_string()))?;
        let mut rows = stmt
            .query_map([], |row| {
                Ok(EncryptedBlob {
                    salt: row.get(0)?,
                    nonce: row.get(1)?,
                    ciphertext: row.get(2)?,
                    vault_id: None,
                    vault_name: None,
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

    pub fn save_vault_for_id(
        &self,
        vault_id: Option<&str>,
        blob: &EncryptedBlob,
    ) -> Result<(), PlatformError> {
        let path = self
            .data_dir()
            .map_err(|e| PlatformError::Path(e.to_string()))?
            .join(vault_file_name(vault_id));
        let conn =
            rusqlite::Connection::open(path).map_err(|e| PlatformError::Storage(e.to_string()))?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS vault_blob (id INTEGER PRIMARY KEY CHECK (id = 1), salt TEXT NOT NULL, nonce TEXT NOT NULL, ciphertext TEXT NOT NULL)",
            [],
        )
        .map_err(|e| PlatformError::Storage(e.to_string()))?;
        conn.execute(
            "INSERT INTO vault_blob (id, salt, nonce, ciphertext) VALUES (1, ?1, ?2, ?3) ON CONFLICT(id) DO UPDATE SET salt = excluded.salt, nonce = excluded.nonce, ciphertext = excluded.ciphertext",
            (&blob.salt, &blob.nonce, &blob.ciphertext),
        )
        .map_err(|e| PlatformError::Storage(e.to_string()))?;
        Ok(())
    }

    pub fn delete_vault_for_id(&self, vault_id: Option<&str>) -> Result<(), PlatformError> {
        let path = self
            .data_dir()
            .map_err(|e| PlatformError::Path(e.to_string()))?
            .join(vault_file_name(vault_id));
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(PlatformError::Storage(format!(
                "failed to delete vault: {error}"
            ))),
        }
    }

    fn settings_path(&self) -> Result<PathBuf, PlatformError> {
        Ok(self
            .config_dir()
            .map_err(|e| PlatformError::Path(e.to_string()))?
            .join("settings.json"))
    }
}

fn vault_file_name(vault_id: Option<&str>) -> String {
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
        let name = blob
            .vault_name
            .as_deref()
            .map(sanitize_backup_name)
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| "vault".to_string());
        let path = backup_dir.join(format!(
            "jpass-backup-{name}-{}.json",
            timestamp.as_millis()
        ));
        let bytes = serde_json::to_vec_pretty(blob)
            .map_err(|e| PlatformError::Storage(format!("failed to serialize backup: {e}")))?;
        fs::write(&path, bytes)
            .map_err(|e| PlatformError::Storage(format!("failed to write backup: {e}")))?;
        Ok(path)
    }

    fn load_encrypted_backup(&self, path: &std::path::Path) -> Result<EncryptedBlob, Self::Error> {
        let bytes = fs::read(path)
            .map_err(|e| PlatformError::Storage(format!("failed to read backup file: {e}")))?;
        serde_json::from_slice(&bytes)
            .map_err(|e| PlatformError::Storage(format!("failed to parse backup file: {e}")))
    }
}

fn sanitize_backup_name(name: &str) -> String {
    name.chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(*ch, '-' | '_'))
        .collect()
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
                vault_id: None,
                vault_name: None,
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
