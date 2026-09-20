use std::collections::HashSet;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VaultFolder {
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub parent_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl VaultFolder {
    pub fn new(name: String) -> Self {
        Self::new_in_parent(name, None)
    }

    pub fn new_in_parent(name: String, parent_id: Option<Uuid>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            parent_id,
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
        title: String,
        username: String,
        password: String,
        url: String,
        notes: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            title,
            username,
            password,
            url,
            notes,
            folder_id: None,
            created_at: now,
            updated_at: now,
        }
    }
}

/// The decrypted, in-memory contents of the password vault.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Vault {
    #[serde(default)]
    pub folders: Vec<VaultFolder>,
    #[serde(default)]
    pub entries: Vec<VaultEntry>,
}

impl Vault {
    pub fn to_json(&self) -> serde_json::Result<Vec<u8>> {
        serde_json::to_vec(self)
    }

    pub fn from_json(bytes: &[u8]) -> serde_json::Result<Self> {
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

    pub fn create_folder(
        &mut self,
        name: &str,
        parent_id: Option<Uuid>,
    ) -> Result<VaultFolder, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Folder name cannot be empty.".into());
        }
        if self
            .folders
            .iter()
            .any(|folder| folder.parent_id == parent_id && folder.name.eq_ignore_ascii_case(name))
        {
            return Err("A folder with that name already exists here.".into());
        }

        if let Some(parent_id) = parent_id {
            if !self.folders.iter().any(|folder| folder.id == parent_id) {
                return Err("Parent folder does not exist.".into());
            }
        }

        let folder = VaultFolder::new_in_parent(name.to_string(), parent_id);
        self.folders.push(folder.clone());
        Ok(folder)
    }

    pub fn rename_folder(&mut self, folder_id: Uuid, new_name: &str) -> Result<(), String> {
        let new_name = new_name.trim();
        if new_name.is_empty() {
            return Err("Folder name cannot be empty.".into());
        }

        let Some(folder) = self
            .folders
            .iter()
            .position(|folder| folder.id == folder_id)
        else {
            return Err("Folder does not exist.".into());
        };

        let parent_id = self.folders[folder].parent_id;
        if self.folders.iter().any(|candidate| {
            candidate.id != folder_id
                && candidate.parent_id == parent_id
                && candidate.name.eq_ignore_ascii_case(new_name)
        }) {
            return Err("A folder with that name already exists here.".into());
        }

        self.folders[folder].name = new_name.to_string();
        self.folders[folder].updated_at = chrono::Utc::now();
        Ok(())
    }

    pub fn folder_is_in_subtree(&self, folder_id: Uuid, ancestor_id: Uuid) -> bool {
        let mut current = Some(folder_id);
        while let Some(id) = current {
            if id == ancestor_id {
                return true;
            }
            current = self
                .folders
                .iter()
                .find(|folder| folder.id == id)
                .and_then(|folder| folder.parent_id);
        }
        false
    }

    pub fn delete_folder(&mut self, folder_id: Uuid) {
        let mut pending = vec![folder_id];
        let mut deleted_folders = HashSet::new();

        while let Some(current_id) = pending.pop() {
            if deleted_folders.insert(current_id) {
                let children: Vec<Uuid> = self
                    .folders
                    .iter()
                    .filter(|folder| folder.parent_id == Some(current_id))
                    .map(|folder| folder.id)
                    .collect();
                pending.extend(children);
            }
        }

        self.folders
            .retain(|folder| !deleted_folders.contains(&folder.id));
        self.entries.retain(|entry| {
            !entry
                .folder_id
                .is_some_and(|id| deleted_folders.contains(&id))
        });
    }

    pub fn move_folder(
        &mut self,
        folder_id: Uuid,
        new_parent_id: Option<Uuid>,
    ) -> Result<(), String> {
        let Some(folder_index) = self
            .folders
            .iter()
            .position(|folder| folder.id == folder_id)
        else {
            return Err("Folder does not exist.".into());
        };

        if let Some(parent_id) = new_parent_id {
            if !self.folders.iter().any(|folder| folder.id == parent_id) {
                return Err("Parent folder does not exist.".into());
            }
            if parent_id == folder_id || self.folder_is_in_subtree(parent_id, folder_id) {
                return Err(
                    "A folder cannot be moved into itself or one of its descendants.".into(),
                );
            }
        }

        let current_parent_id = self.folders[folder_index].parent_id;
        let target_parent_id = new_parent_id;
        let target_name = self.folders[folder_index].name.clone();

        if self.folders.iter().any(|candidate| {
            candidate.id != folder_id
                && candidate.parent_id == target_parent_id
                && candidate.name.eq_ignore_ascii_case(&target_name)
        }) {
            return Err("A folder with that name already exists here.".into());
        }

        self.folders[folder_index].parent_id = target_parent_id;
        self.folders[folder_index].updated_at = Utc::now();

        if current_parent_id == target_parent_id {
            return Ok(());
        }

        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vault_json_round_trip_preserves_entries() {
        let now = Utc::now();
        let mut vault = Vault::default();
        vault.add_entry(VaultEntry {
            id: Uuid::new_v4(),
            title: "GitHub".into(),
            username: "alice".into(),
            password: "Secret123!".into(),
            url: "https://github.com".into(),
            notes: "Primary account".into(),
            folder_id: None,
            created_at: now,
            updated_at: now,
        });

        let bytes = vault.to_json().unwrap();
        let restored = Vault::from_json(&bytes).unwrap();

        assert_eq!(restored.entries.len(), 1);
        assert_eq!(restored.entries[0].title, "GitHub");
        assert_eq!(restored.entries[0].username, "alice");
    }

    #[test]
    fn vault_can_create_and_move_folders() {
        let mut vault = Vault::default();
        let first = VaultEntry::new(
            "GitHub".into(),
            "alice".into(),
            "secret".into(),
            "https://github.com".into(),
            "work".into(),
        );
        let second = VaultEntry::new(
            "Bank".into(),
            "alice".into(),
            "secret2".into(),
            "https://bank.example".into(),
            "finance".into(),
        );
        let id = first.id;
        vault.add_entry(first);
        vault.add_entry(second);

        let folder = vault.create_folder("Work", None).unwrap();
        assert!(vault.move_entry_to_folder(id, Some(folder.id)));
        assert_eq!(vault.entries[0].folder_id, Some(folder.id));

        let moved = vault.entries.iter().find(|e| e.id == id).unwrap();
        assert_eq!(moved.folder_id, Some(folder.id));
    }

    #[test]
    fn vault_supports_nested_folders_and_sibling_names() {
        let mut vault = Vault::default();
        let personal = vault.create_folder("Personal", None).unwrap();
        let finances = vault.create_folder("Finances", Some(personal.id)).unwrap();

        assert_eq!(finances.parent_id, Some(personal.id));
        assert!(vault.folder_is_in_subtree(finances.id, personal.id));
        assert!(vault.create_folder("Finances", Some(personal.id)).is_err());
        assert!(vault.create_folder("Finances", None).is_ok());
    }

    #[test]
    fn vault_can_rename_folders() {
        let mut vault = Vault::default();
        let personal = vault.create_folder("Personal", None).unwrap();
        let finances = vault.create_folder("Finances", Some(personal.id)).unwrap();
        let banking = vault.create_folder("Banking", Some(personal.id)).unwrap();

        assert!(vault.rename_folder(personal.id, "Work").is_ok());
        assert!(vault.rename_folder(finances.id, "Banking").is_err());
        assert!(vault.rename_folder(banking.id, "Savings").is_ok());

        let updated_personal = vault
            .folders
            .iter()
            .find(|folder| folder.id == personal.id)
            .unwrap();
        assert_eq!(updated_personal.name, "Work");
    }

    #[test]
    fn vault_can_move_folder_to_another_parent() {
        let mut vault = Vault::default();
        let personal = vault.create_folder("Personal", None).unwrap();
        let finances = vault.create_folder("Finances", Some(personal.id)).unwrap();
        let banking = vault.create_folder("Banking", Some(personal.id)).unwrap();

        assert!(vault.move_folder(finances.id, None).is_ok());
        assert_eq!(
            vault
                .folders
                .iter()
                .find(|folder| folder.id == finances.id)
                .unwrap()
                .parent_id,
            None
        );
        assert!(vault.move_folder(personal.id, Some(banking.id)).is_err());
    }

    #[test]
    fn vault_can_delete_folder_and_entries_in_it() {
        let mut vault = Vault::default();
        let personal = vault.create_folder("Personal", None).unwrap();
        let finances = vault.create_folder("Finances", Some(personal.id)).unwrap();
        let work = VaultEntry::new(
            "GitHub".into(),
            "alice".into(),
            "secret".into(),
            "https://github.com".into(),
            "work".into(),
        );
        let mut bank = VaultEntry::new(
            "Bank".into(),
            "alice".into(),
            "secret2".into(),
            "https://bank.example".into(),
            "finance".into(),
        );
        bank.folder_id = Some(finances.id);
        vault.add_entry(work);
        vault.add_entry(bank);

        vault.delete_folder(personal.id);

        assert!(!vault.folders.iter().any(|folder| folder.id == personal.id));
        assert!(!vault.folders.iter().any(|folder| folder.id == finances.id));
        assert!(vault
            .entries
            .iter()
            .all(|entry| entry.folder_id != Some(finances.id)));
    }
}
