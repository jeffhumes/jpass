use jni::objects::{JObject, JString};
use jni::JavaVM;
use jpass_core::{AppSettings, EncryptedBlob};
use jpass_platform::{BackupService, ClipboardService, PlatformError, PlatformPaths, VaultStore};
use std::fs;
use std::io::ErrorKind;
use std::path::Path;
use std::path::PathBuf;

pub struct AndroidStore;

impl VaultStore for AndroidStore {
    type Error = PlatformError;

    fn load_vault(&self) -> Result<Option<EncryptedBlob>, Self::Error> {
        self.load_vault_for_id(None)
    }

    fn save_vault(&self, blob: &EncryptedBlob) -> Result<(), Self::Error> {
        self.save_vault_for_id(None, blob)
    }

    fn load_settings(&self) -> Result<AppSettings, Self::Error> {
        let path = self.config_dir()?.join("settings.json");
        match fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(|error| {
                PlatformError::Storage(format!("failed to parse settings: {error}"))
            }),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(AppSettings::default()),
            Err(error) => Err(PlatformError::Storage(format!(
                "failed to read settings: {error}"
            ))),
        }
    }

    fn save_settings(&self, settings: &AppSettings) -> Result<(), Self::Error> {
        let bytes = serde_json::to_vec_pretty(settings).map_err(|error| {
            PlatformError::Storage(format!("failed to serialize settings: {error}"))
        })?;
        write_atomically(&self.config_dir()?.join("settings.json"), &bytes)
    }

    fn delete_vault_for_id(&self, vault_id: Option<&str>) -> Result<(), Self::Error> {
        match fs::remove_file(self.vault_path(vault_id)?) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
            Err(error) => Err(PlatformError::Storage(format!(
                "failed to delete vault: {error}"
            ))),
        }
    }
}

impl AndroidStore {
    pub fn load_vault_for_id(
        &self,
        vault_id: Option<&str>,
    ) -> Result<Option<EncryptedBlob>, PlatformError> {
        match fs::read(self.vault_path(vault_id)?) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map(Some)
                .map_err(|error| PlatformError::Storage(format!("failed to parse vault: {error}"))),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
            Err(error) => Err(PlatformError::Storage(format!(
                "failed to read vault: {error}"
            ))),
        }
    }

    pub fn save_vault_for_id(
        &self,
        vault_id: Option<&str>,
        blob: &EncryptedBlob,
    ) -> Result<(), PlatformError> {
        let bytes = serde_json::to_vec(blob).map_err(|error| {
            PlatformError::Storage(format!("failed to serialize vault: {error}"))
        })?;
        write_atomically(&self.vault_path(vault_id)?, &bytes)
    }

    fn vault_path(&self, vault_id: Option<&str>) -> Result<PathBuf, PlatformError> {
        Ok(self.data_dir()?.join(vault_file_name(vault_id)))
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

    fn save_encrypted_backup(
        &self,
        blob: &EncryptedBlob,
        destination_dir: Option<&std::path::Path>,
    ) -> Result<PathBuf, Self::Error> {
        let backup_dir = match destination_dir {
            Some(dir) => dir.to_path_buf(),
            None => self.data_dir()?.join("backups"),
        };
        fs::create_dir_all(&backup_dir).map_err(|error| {
            PlatformError::Storage(format!("failed to create backup directory: {error}"))
        })?;
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|error| {
                PlatformError::Storage(format!("failed to determine backup timestamp: {error}"))
            })?;
        let name = blob
            .vault_name
            .as_deref()
            .map(sanitize_name)
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| "vault".to_string());
        let path = backup_dir.join(format!(
            "jpass-backup-{name}-{}.json",
            timestamp.as_millis()
        ));
        let bytes = serde_json::to_vec_pretty(blob).map_err(|error| {
            PlatformError::Storage(format!("failed to serialize backup: {error}"))
        })?;
        write_atomically(&path, &bytes)?;
        Ok(path)
    }

    fn load_encrypted_backup(&self, path: &std::path::Path) -> Result<EncryptedBlob, Self::Error> {
        let bytes = fs::read(path).map_err(|error| {
            PlatformError::Storage(format!("failed to read backup file: {error}"))
        })?;
        serde_json::from_slice(&bytes).map_err(|error| {
            PlatformError::Storage(format!("failed to parse backup file: {error}"))
        })
    }
}

impl PlatformPaths for AndroidStore {
    type Error = PlatformError;

    fn data_dir(&self) -> Result<PathBuf, Self::Error> {
        app_dir()
    }

    fn config_dir(&self) -> Result<PathBuf, Self::Error> {
        app_dir()
    }
}

pub fn android_store() -> AndroidStore {
    AndroidStore
}

fn app_dir() -> Result<PathBuf, PlatformError> {
    let context = ndk_context::android_context();
    let java_vm = unsafe { JavaVM::from_raw(context.vm().cast()) }
        .map_err(|error| PlatformError::Path(format!("failed to access Android VM: {error}")))?;
    let mut env = java_vm.attach_current_thread().map_err(|error| {
        PlatformError::Path(format!("failed to attach Android storage thread: {error}"))
    })?;
    let context = unsafe { JObject::from_raw(context.context().cast()) };
    let files_dir = env
        .call_method(context, "getFilesDir", "()Ljava/io/File;", &[])
        .and_then(|value| value.l())
        .map_err(|error| {
            PlatformError::Path(format!(
                "failed to resolve Android files directory: {error}"
            ))
        })?;
    let path = env
        .call_method(files_dir, "getAbsolutePath", "()Ljava/lang/String;", &[])
        .and_then(|value| value.l())
        .map_err(|error| {
            PlatformError::Path(format!("failed to resolve Android files path: {error}"))
        })?;
    let path = JString::from(path);
    let dir = PathBuf::from(
        env.get_string(&path)
            .map_err(|error| {
                PlatformError::Path(format!("failed to read Android files path: {error}"))
            })?
            .to_string_lossy()
            .into_owned(),
    )
    .join(".jpass");
    fs::create_dir_all(&dir).map_err(|error| {
        PlatformError::Path(format!("failed to create app data directory: {error}"))
    })?;
    Ok(dir)
}

fn vault_file_name(vault_id: Option<&str>) -> String {
    match vault_id.map(sanitize_name).filter(|id| !id.is_empty()) {
        Some(id) => format!("vault-{id}.json"),
        None => "vault.json".to_string(),
    }
}

fn sanitize_name(name: &str) -> String {
    name.chars()
        .filter(|character| character.is_ascii_alphanumeric() || matches!(*character, '-' | '_'))
        .collect()
}

fn write_atomically(path: &Path, bytes: &[u8]) -> Result<(), PlatformError> {
    let temporary_path = path.with_extension("tmp");
    fs::write(&temporary_path, bytes)
        .map_err(|error| PlatformError::Storage(format!("failed to write data: {error}")))?;
    fs::rename(&temporary_path, path)
        .map_err(|error| PlatformError::Storage(format!("failed to finalize data: {error}")))
}
