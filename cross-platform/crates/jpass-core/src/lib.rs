use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppSettings {
    pub clipboard_timeout_secs: u64,
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

    pub fn create_folder(&mut self, name: &str) -> Result<VaultFolder, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Folder name cannot be empty.".to_string());
        }
        if self
            .folders
            .iter()
            .any(|folder| folder.name.eq_ignore_ascii_case(name))
        {
            return Err("Folder already exists.".to_string());
        }

        let folder = VaultFolder::new(name.to_string());
        self.folders.push(folder.clone());
        Ok(folder)
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
    #[error("vault is unavailable")]
    Unavailable,
}

pub fn sample_vault() -> Vault {
    let mut vault = Vault::default();
    vault.add_entry(VaultEntry::new(
        "GitHub",
        "alice@example.com",
        "super-secret",
        "https://github.com",
        "Primary development account",
    ));
    vault
}
