use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppTheme {
    Dark,
    Light,
    System,
}

impl Default for AppTheme {
    fn default() -> Self {
        Self::Dark
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntryActionDisplay {
    Text,
    Icons,
}

impl Default for EntryActionDisplay {
    fn default() -> Self {
        Self::Text
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrimaryActionDisplay {
    Text,
    Icons,
}

impl Default for PrimaryActionDisplay {
    fn default() -> Self {
        Self::Text
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToastPosition {
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

impl Default for ToastPosition {
    fn default() -> Self {
        Self::BottomRight
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditPasswordGenerationMode {
    AutoGenerate,
    FullGenerator,
}

impl Default for EditPasswordGenerationMode {
    fn default() -> Self {
        Self::AutoGenerate
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub clipboard_timeout_secs: u64,
    #[serde(default)]
    pub theme: AppTheme,
    #[serde(default = "default_confirm_delete")]
    pub confirm_delete: bool,
    #[serde(default)]
    pub entry_action_display: EntryActionDisplay,
    #[serde(default)]
    pub primary_action_display: PrimaryActionDisplay,
    #[serde(default = "default_true")]
    pub show_primary_action_icons: bool,
    #[serde(default)]
    pub vaults: Vec<VaultProfile>,
    #[serde(default)]
    pub active_vault_id: Option<String>,
    #[serde(default)]
    pub default_vault_id: Option<String>,
    #[serde(default)]
    pub toast_position: ToastPosition,
    #[serde(default = "default_generator_length")]
    pub generator_length: usize,
    #[serde(default = "default_true")]
    pub generator_lowercase: bool,
    #[serde(default = "default_true")]
    pub generator_uppercase: bool,
    #[serde(default = "default_true")]
    pub generator_digits: bool,
    #[serde(default = "default_true")]
    pub generator_symbols: bool,
    #[serde(default)]
    pub edit_password_generation_mode: EditPasswordGenerationMode,
    #[serde(default)]
    pub sync_enabled: bool,
    #[serde(default)]
    pub sync_folder: Option<String>,
    #[serde(default = "default_sync_device_id")]
    pub sync_device_id: String,
    #[serde(default)]
    pub sync_revision: u64,
    #[serde(default)]
    pub last_sync_at: Option<DateTime<Utc>>,
}

fn default_confirm_delete() -> bool {
    true
}

fn default_generator_length() -> usize {
    20
}

fn default_true() -> bool {
    true
}

fn default_sync_device_id() -> String {
    Uuid::new_v4().to_string()
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            clipboard_timeout_secs: Self::default_timeout(),
            theme: AppTheme::default(),
            confirm_delete: true,
            entry_action_display: EntryActionDisplay::default(),
            primary_action_display: PrimaryActionDisplay::default(),
            show_primary_action_icons: true,
            vaults: Vec::new(),
            active_vault_id: None,
            default_vault_id: None,
            toast_position: ToastPosition::default(),
            generator_length: default_generator_length(),
            generator_lowercase: true,
            generator_uppercase: true,
            generator_digits: true,
            generator_symbols: true,
            edit_password_generation_mode: EditPasswordGenerationMode::default(),
            sync_enabled: false,
            sync_folder: None,
            sync_device_id: default_sync_device_id(),
            sync_revision: 0,
            last_sync_at: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_settings_tracks_multiple_vaults() {
        let default_settings = AppSettings::default();
        assert!(!default_settings.has_active_vault());

        let primary = VaultProfile {
            id: "vault-primary".to_string(),
            name: "Primary".to_string(),
        };
        let work = VaultProfile {
            id: "vault-work".to_string(),
            name: "Work".to_string(),
        };

        let mut updated = AppSettings::default();
        updated.vaults = vec![primary.clone(), work.clone()];
        updated.active_vault_id = Some(primary.id.clone());
        updated.default_vault_id = Some(primary.id.clone());

        assert!(updated.has_active_vault());
        assert_eq!(updated.vaults.len(), 2);
        assert_eq!(
            updated.active_vault_id.as_deref(),
            Some(primary.id.as_str())
        );
    }
}

impl AppSettings {
    pub fn default_timeout() -> u64 {
        10
    }

    pub fn has_active_vault(&self) -> bool {
        self.active_vault_id.is_some() || !self.vaults.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VaultProfile {
    pub id: String,
    pub name: String,
}

impl Default for VaultProfile {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: "Primary".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VaultFolder {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl VaultFolder {
    pub fn new(name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VaultEntry {
    pub id: Uuid,
    pub title: String,
    pub username: String,
    pub password: String,
    pub url: String,
    pub notes: String,
    #[serde(default)]
    pub folder_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl VaultEntry {
    pub fn new(
        title: impl Into<String>,
        username: impl Into<String>,
        password: impl Into<String>,
        url: impl Into<String>,
        notes: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            title: title.into(),
            username: username.into(),
            password: password.into(),
            url: url.into(),
            notes: notes.into(),
            folder_id: None,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Vault {
    #[serde(default)]
    pub folders: Vec<VaultFolder>,
    #[serde(default)]
    pub entries: Vec<VaultEntry>,
}

impl Vault {
    pub fn to_json(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    pub fn from_json(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }

    pub fn add_entry(&mut self, entry: VaultEntry) {
        self.entries.push(entry);
    }

    pub fn update_entry(&mut self, updated: VaultEntry) {
        if let Some(existing) = self.entries.iter_mut().find(|e| e.id == updated.id) {
            *existing = updated;
        }
    }

    pub fn remove_entry(&mut self, id: Uuid) {
        self.entries.retain(|e| e.id != id);
    }

    pub fn create_folder(&mut self, name: &str) -> Result<VaultFolder, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Folder name cannot be empty.".into());
        }
        if self
            .folders
            .iter()
            .any(|folder| folder.name.eq_ignore_ascii_case(name))
        {
            return Err("Folder already exists.".into());
        }

        let folder = VaultFolder::new(name.to_string());
        self.folders.push(folder.clone());
        Ok(folder)
    }

    pub fn move_entry_to_folder(&mut self, entry_id: Uuid, folder_id: Option<Uuid>) -> bool {
        let Some(entry) = self.entries.iter_mut().find(|entry| entry.id == entry_id) else {
            return false;
        };

        if folder_id.is_some()
            && !self
                .folders
                .iter()
                .any(|folder| folder.id == folder_id.unwrap())
        {
            return false;
        }

        entry.folder_id = folder_id;
        entry.updated_at = Utc::now();
        true
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EncryptedBlob {
    pub salt: String,
    pub nonce: String,
    pub ciphertext: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyncEnvelope {
    pub schema_version: u32,
    pub device_id: String,
    pub revision: u64,
    pub modified_at: DateTime<Utc>,
    pub vault: EncryptedBlob,
}

impl SyncEnvelope {
    pub const CURRENT_SCHEMA_VERSION: u32 = 1;

    pub fn new(device_id: impl Into<String>, revision: u64, vault: EncryptedBlob) -> Self {
        Self {
            schema_version: Self::CURRENT_SCHEMA_VERSION,
            device_id: device_id.into(),
            revision,
            modified_at: Utc::now(),
            vault,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum VaultError {
    #[error("vault serialization failed: {0}")]
    Serialization(String),
    #[error("vault deserialization failed: {0}")]
    Deserialization(String),
    #[error("vault unavailable")]
    Unavailable,
}
