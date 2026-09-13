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
    #[serde(default)]
    pub toast_position: ToastPosition,
}

fn default_confirm_delete() -> bool {
    true
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            clipboard_timeout_secs: Self::default_timeout(),
            theme: AppTheme::default(),
            confirm_delete: true,
            entry_action_display: EntryActionDisplay::default(),
            primary_action_display: PrimaryActionDisplay::default(),
            toast_position: ToastPosition::default(),
        }
    }
}

impl AppSettings {
    pub fn default_timeout() -> u64 {
        10
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedBlob {
    pub salt: String,
    pub nonce: String,
    pub ciphertext: String,
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
