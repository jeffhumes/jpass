use crate::crypto::{self, EncryptedBlob};
use crate::model::{Vault, VaultEntry, VaultFolder};
use crate::password_gen::{generate_password, PasswordOptions};
use crate::{clipboard, storage};
use dioxus::prelude::*;
use jpass_core::{
    AppSettings, AppTheme, EditPasswordGenerationMode, EntryActionDisplay, PrimaryActionDisplay,
    SyncEnvelope, ToastPosition, VaultProfile,
};
use std::collections::HashSet;
use uuid::Uuid;

const MAIN_CSS: &str = include_str!("../assets/main.css");

#[derive(Clone, Copy, PartialEq)]
enum Screen {
    Loading,
    SelectVault,
    CreateMaster,
    Unlock,
    Vault,
}

fn active_vault_id() -> Option<String> {
    storage::load_settings()
        .ok()
        .and_then(|settings| settings.active_vault_id)
}

fn prepare_vault_startup(error: &mut Signal<Option<String>>) -> Screen {
    let mut settings = storage::load_settings().unwrap_or_default();

    if settings.vaults.is_empty() {
        match storage::load_encrypted_for_vault(None) {
            Ok(Some(blob)) => {
                let vault_id = uuid::Uuid::new_v4().to_string();
                settings.vaults.push(VaultProfile {
                    id: vault_id.clone(),
                    name: "Primary".to_string(),
                });
                settings.active_vault_id = Some(vault_id.clone());
                settings.default_vault_id = Some(vault_id.clone());

                if let Err(storage_error) =
                    storage::save_encrypted_for_vault(Some(&vault_id), &blob)
                        .and_then(|_| storage::save_settings(&settings))
                {
                    error.set(Some(format!(
                        "Failed to migrate the existing vault: {storage_error}"
                    )));
                    return Screen::CreateMaster;
                }
            }
            Ok(None) => return Screen::CreateMaster,
            Err(storage_error) => {
                error.set(Some(format!("Storage error: {storage_error}")));
                return Screen::CreateMaster;
            }
        }
    }

    let default_is_valid = settings
        .default_vault_id
        .as_ref()
        .is_some_and(|default_id| {
            settings
                .vaults
                .iter()
                .any(|profile| profile.id == *default_id)
        });
    if !default_is_valid {
        settings.default_vault_id = settings
            .active_vault_id
            .as_ref()
            .filter(|active_id| {
                settings
                    .vaults
                    .iter()
                    .any(|profile| profile.id == **active_id)
            })
            .cloned()
            .or_else(|| settings.vaults.first().map(|profile| profile.id.clone()));
        if let Err(storage_error) = storage::save_settings(&settings) {
            error.set(Some(format!(
                "Failed to save the default vault: {storage_error}"
            )));
        }
    }

    Screen::SelectVault
}

#[derive(Clone, Debug)]
struct ToastState {
    id: u64,
    label: String,
    duration_ms: u64,
    remaining_ms: u64,
}

fn flatten_folder_tree(folders: &[VaultFolder]) -> Vec<(VaultFolder, usize)> {
    fn append_children(
        folders: &[VaultFolder],
        parent_id: Option<Uuid>,
        depth: usize,
        result: &mut Vec<(VaultFolder, usize)>,
    ) {
        for folder in folders
            .iter()
            .filter(|folder| folder.parent_id == parent_id)
        {
            result.push((folder.clone(), depth));
            append_children(folders, Some(folder.id), depth + 1, result);
        }
    }

    let mut result = Vec::new();
    append_children(folders, None, 0, &mut result);
    result
}

fn folder_path(folders: &[VaultFolder], folder_id: Uuid) -> String {
    let mut names = Vec::new();
    let mut current_id = Some(folder_id);
    let mut guard = 0;

    while let Some(id) = current_id {
        let Some(folder) = folders.iter().find(|folder| folder.id == id) else {
            break;
        };
        names.push(folder.name.clone());
        current_id = folder.parent_id;
        guard += 1;
        if guard > folders.len() {
            break;
        }
    }

    names.reverse();
    names.join(" / ")
}

fn folder_has_children(folders: &[VaultFolder], folder_id: Uuid) -> bool {
    folders
        .iter()
        .any(|folder| folder.parent_id == Some(folder_id))
}

fn folder_ancestors(folders: &[VaultFolder], folder_id: Uuid) -> Vec<Uuid> {
    let mut ancestors = Vec::new();
    let mut current = Some(folder_id);

    while let Some(id) = current {
        let Some(folder) = folders.iter().find(|folder| folder.id == id) else {
            break;
        };
        if let Some(parent_id) = folder.parent_id {
            ancestors.push(parent_id);
            current = Some(parent_id);
        } else {
            break;
        }
    }

    ancestors
}

fn folder_is_visible(folders: &[VaultFolder], folder_id: Uuid, expanded: &HashSet<Uuid>) -> bool {
    let Some(folder) = folders.iter().find(|folder| folder.id == folder_id) else {
        return false;
    };
    let Some(parent_id) = folder.parent_id else {
        return true;
    };
    expanded.contains(&parent_id) && folder_is_visible(folders, parent_id, expanded)
}

enum ToastCommand {
    Show(ToastState),
    Cancel(u64),
}

#[allow(non_snake_case)]
pub fn App() -> Element {
    let mut screen = use_signal(|| Screen::Loading);
    let vault = use_signal::<Option<Vault>>(|| None);
    let master_password = use_signal::<Option<String>>(|| None);
    let mut error = use_signal::<Option<String>>(|| None);

    // Resolve the active profile before showing the unlock screen.
    use_effect(move || {
        if matches!(screen(), Screen::Loading) {
            screen.set(prepare_vault_startup(&mut error));
        }
    });

    rsx! {
        document::Title { "JPass" }
        style { {MAIN_CSS} }
        div { class: "app",
            match screen() {
                Screen::Loading => rsx! { p { "Loading…" } },
                Screen::SelectVault => rsx! {
                    SelectVaultScreen { screen, vault, master_password, error }
                },
                Screen::CreateMaster => rsx! {
                    CreateMasterScreen { screen, vault, master_password, error }
                },
                Screen::Unlock => rsx! {
                    UnlockScreen { screen, vault, master_password, error }
                },
                Screen::Vault => rsx! {
                    VaultScreen { screen, vault, master_password }
                },
            }
        }
    }
}

#[component]
fn SelectVaultScreen(
    screen: Signal<Screen>,
    vault: Signal<Option<Vault>>,
    master_password: Signal<Option<String>>,
    error: Signal<Option<String>>,
) -> Element {
    let mut settings = use_signal(|| storage::load_settings().unwrap_or_default());
    let profile_list = settings().vaults.clone();

    let mut choose = move |vault_id: String| {
        let mut updated = settings();
        updated.active_vault_id = Some(vault_id.clone());
        if storage::save_settings(&updated).is_ok() {
            settings.set(updated);
            vault.set(None);
            master_password.set(None);
            screen.set(Screen::Unlock);
        } else {
            error.set(Some("Failed to save the selected vault.".into()));
        }
    };

    let mut set_default = move |vault_id: String| {
        let mut updated = settings();
        updated.default_vault_id = Some(vault_id);
        match storage::save_settings(&updated) {
            Ok(()) => settings.set(updated),
            Err(_) => error.set(Some("Failed to save the default vault.".into())),
        }
    };

    rsx! {
        div { class: "centered-card",
            h1 { "Choose a vault" }
            p { "Select which vault you want to unlock." }
            div { class: "vault-picker-list",
                for profile in profile_list {
                    div { class: "vault-option",
                        button {
                            class: "secondary-btn",
                            onclick: {
                                let vault_id = profile.id.clone();
                                move |_| choose(vault_id.clone())
                            },
                            "{profile.name}"
                        }
                        label {
                            input {
                                r#type: "radio",
                                name: "default-vault",
                                checked: settings().default_vault_id.as_deref() == Some(profile.id.as_str()),
                                onchange: {
                                    let vault_id = profile.id.clone();
                                    move |_| set_default(vault_id.clone())
                                },
                            }
                            " Default"
                        }
                    }
                }
            }
            button {
                class: "primary",
                onclick: move |_| screen.set(Screen::CreateMaster),
                "Create a new vault"
            }
        }
    }
}

#[component]
fn CreateMasterScreen(
    screen: Signal<Screen>,
    vault: Signal<Option<Vault>>,
    master_password: Signal<Option<String>>,
    error: Signal<Option<String>>,
) -> Element {
    let mut vault_name = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut confirm = use_signal(String::new);
    let mut screen = screen;
    let mut vault = vault;
    let mut master_password = master_password;
    let mut error = error;

    let mut submit = move || {
        let pw = password();
        if pw.len() < 8 {
            error.set(Some(
                "Master password must be at least 8 characters.".into(),
            ));
            return;
        }
        if pw != confirm() {
            error.set(Some("Passwords do not match.".into()));
            return;
        }
        let name = vault_name().trim().to_string();
        if name.is_empty() {
            error.set(Some("Vault name cannot be empty.".into()));
            return;
        }
        let new_vault = Vault::default();
        let mut settings = storage::load_settings().unwrap_or_default();
        let vault_id = uuid::Uuid::new_v4().to_string();
        settings.vaults.push(VaultProfile {
            id: vault_id.clone(),
            name,
        });
        settings.active_vault_id = Some(vault_id.clone());
        if settings.default_vault_id.is_none() {
            settings.default_vault_id = Some(vault_id.clone());
        }
        let json = new_vault.to_json().expect("vault serializes");
        match crypto::encrypt(&json, &pw) {
            Ok(blob) => {
                if let Err(e) = storage::save_encrypted_for_vault(Some(&vault_id), &blob) {
                    error.set(Some(format!("Failed to save vault: {e}")));
                    return;
                }
                if let Err(e) = storage::save_settings(&settings) {
                    error.set(Some(format!("Failed to save vault metadata: {e}")));
                    return;
                }
                vault.set(Some(new_vault));
                master_password.set(Some(pw));
                error.set(None);
                screen.set(Screen::Vault);
            }
            Err(_) => error.set(Some("Failed to encrypt vault.".into())),
        }
    };

    rsx! {
        div { class: "centered-card",
            h1 { "Welcome to JPass" }
            p { "Name your vault and create a master password to protect it." }
            input {
                placeholder: "Vault name",
                value: "{vault_name}",
                oninput: move |e| vault_name.set(e.value()),
            }
            input {
                r#type: "password",
                placeholder: "Master password",
                value: "{password}",
                oninput: move |e| password.set(e.value()),
            }
            input {
                r#type: "password",
                placeholder: "Confirm master password",
                value: "{confirm}",
                oninput: move |e| confirm.set(e.value()),
                onkeydown: move |e| { if e.key() == Key::Enter { submit(); } },
            }
            if let Some(msg) = error() {
                p { class: "error", "{msg}" }
            }
            button { onclick: move |_| submit(), "Create Vault" }
        }
    }
}

#[component]
fn UnlockScreen(
    screen: Signal<Screen>,
    vault: Signal<Option<Vault>>,
    master_password: Signal<Option<String>>,
    error: Signal<Option<String>>,
) -> Element {
    let mut password = use_signal(String::new);
    let mut screen = screen;
    let mut vault = vault;
    let mut master_password = master_password;
    let mut error = error;

    let mut submit = move || {
        let pw = password();
        let active_id = storage::load_settings()
            .ok()
            .and_then(|settings| settings.active_vault_id.clone());
        let blob: EncryptedBlob = match storage::load_encrypted_for_vault(active_id.as_deref()) {
            Ok(Some(b)) => b,
            Ok(None) => {
                error.set(Some("No vault found.".into()));
                return;
            }
            Err(e) => {
                error.set(Some(format!("Storage error: {e}")));
                return;
            }
        };
        match crypto::decrypt(&blob, &pw) {
            Ok(json) => match Vault::from_json(&json) {
                Ok(v) => {
                    vault.set(Some(v));
                    master_password.set(Some(pw));
                    error.set(None);
                    screen.set(Screen::Vault);
                }
                Err(_) => error.set(Some("Vault data is corrupted.".into())),
            },
            Err(_) => error.set(Some("Incorrect master password.".into())),
        }
    };

    rsx! {
        div { class: "centered-card",
            h1 { "JPass" }
            p { "Enter your master password to unlock the vault." }
            input {
                r#type: "password",
                placeholder: "Master password",
                value: "{password}",
                oninput: move |e| password.set(e.value()),
                onkeydown: move |e| { if e.key() == Key::Enter { submit(); } },
                autofocus: true,
            }
            if let Some(msg) = error() {
                p { class: "error", "{msg}" }
            }
            button { onclick: move |_| submit(), "Unlock" }
        }
    }
}

#[component]
fn VaultScreen(
    screen: Signal<Screen>,
    vault: Signal<Option<Vault>>,
    master_password: Signal<Option<String>>,
) -> Element {
    let mut screen = screen;
    let mut vault = vault;
    let mut master_password = master_password;
    let mut search = use_signal(String::new);
    let mut selected_folder = use_signal::<Option<Uuid>>(|| None);
    let mut expanded_folders = use_signal(HashSet::<Uuid>::new);
    let mut expanded_initialized = use_signal(|| false);
    let mut editing = use_signal::<Option<VaultEntry>>(|| None);
    let mut show_editor = use_signal(|| false);
    let mut show_folder_modal = use_signal(|| false);
    let mut show_folder_rename_modal = use_signal(|| false);
    let mut show_settings = use_signal(|| false);
    let mut show_generator = use_signal(|| false);
    let mut show_entry_generator = use_signal(|| false);
    let mut generated_entry_password = use_signal::<Option<String>>(|| None);
    let mut pending_delete = use_signal::<Option<VaultEntry>>(|| None);
    let mut pending_folder_delete = use_signal::<Option<VaultFolder>>(|| None);
    let mut pending_folder_move = use_signal::<Option<VaultFolder>>(|| None);
    let mut pending_move = use_signal::<Option<VaultEntry>>(|| None);
    let mut new_folder_name = use_signal(String::new);
    let mut new_folder_parent_id = use_signal(String::new);
    let mut rename_folder_id = use_signal::<Option<Uuid>>(|| None);
    let mut rename_folder_name = use_signal(String::new);
    let mut folder_context_menu = use_signal::<Option<Uuid>>(|| None);
    let mut save_error = use_signal::<Option<String>>(|| None);
    let mut notification = use_signal::<Vec<ToastState>>(Vec::new);
    let mut settings = use_signal(|| storage::load_settings().unwrap_or_default());
    let mut show_vault_switcher = use_signal(|| false);

    let copy_timer = use_coroutine(
        move |mut commands: UnboundedReceiver<ToastCommand>| async move {
            let mut latest_toast_id = 0u64;
            loop {
                while let Ok(command) = commands.try_recv() {
                    match command {
                        ToastCommand::Cancel(id) => {
                            notification.write().retain(|toast| toast.id != id)
                        }
                        ToastCommand::Show(mut current) => {
                            current.id = notification()
                                .iter()
                                .map(|toast| toast.id)
                                .max()
                                .unwrap_or(0)
                                + 1;
                            latest_toast_id = current.id;
                            notification.write().push(current);
                        }
                    }
                }

                futures_timer::Delay::new(std::time::Duration::from_millis(50)).await;
                let mut should_clear_clipboard = false;
                let mut updated = notification();
                for toast in &mut updated {
                    toast.remaining_ms = toast.remaining_ms.saturating_sub(50);
                    if toast.remaining_ms == 0 && toast.id == latest_toast_id {
                        should_clear_clipboard = true;
                    }
                }
                updated.retain(|toast| toast.remaining_ms > 0);
                if should_clear_clipboard {
                    clipboard::clear_clipboard();
                }
                notification.set(updated);
            }
        },
    );

    let persist = move |v: &Vault| -> Result<(), String> {
        let Some(pw) = master_password() else {
            return Err("Vault is locked.".into());
        };
        let json = v.to_json().map_err(|e| e.to_string())?;
        let blob = crypto::encrypt(&json, &pw).map_err(|_| "encryption failed".to_string())?;
        let active_id = active_vault_id();
        storage::save_encrypted_for_vault(active_id.as_deref(), &blob).map_err(|e| e.to_string())
    };

    let mut delete_entry = move |id: Uuid| {
        if let Some(mut v) = vault() {
            v.remove_entry(id);
            match persist(&v) {
                Ok(()) => {
                    vault.set(Some(v));
                    save_error.set(None);
                }
                Err(error) => save_error.set(Some(error)),
            }
        }
    };

    let mut delete_folder = move |id: Uuid| {
        if let Some(mut v) = vault() {
            v.delete_folder(id);
            match persist(&v) {
                Ok(()) => {
                    vault.set(Some(v));
                    save_error.set(None);
                    selected_folder.set(None);
                }
                Err(error) => save_error.set(Some(error)),
            }
        }
    };

    let mut move_folder = move |folder_id: Uuid, new_parent_id: Option<Uuid>| {
        if let Some(mut v) = vault() {
            match v.move_folder(folder_id, new_parent_id) {
                Ok(()) => match persist(&v) {
                    Ok(()) => {
                        vault.set(Some(v));
                        save_error.set(None);
                    }
                    Err(error) => save_error.set(Some(error)),
                },
                Err(error) => save_error.set(Some(error)),
            }
        }
    };

    let mut move_entry =
        move |(entry_id, folder_id, new_folder_name): (Uuid, Option<Uuid>, Option<String>)| {
            if let Some(mut v) = vault() {
                let target_folder = if let Some(name) = new_folder_name {
                    match v.create_folder(&name, folder_id) {
                        Ok(folder) => Some(folder.id),
                        Err(error) => {
                            save_error.set(Some(error));
                            return;
                        }
                    }
                } else {
                    folder_id
                };

                if v.move_entry_to_folder(entry_id, target_folder) {
                    match persist(&v) {
                        Ok(()) => {
                            vault.set(Some(v));
                            save_error.set(None);
                        }
                        Err(error) => save_error.set(Some(error)),
                    }
                }
            }
        };

    let create_backup = move || -> Result<String, String> {
        let Some(pw) = master_password() else {
            return Err("Vault is locked.".into());
        };
        let Some(current_vault) = vault() else {
            return Err("Vault is unavailable.".into());
        };
        let json = current_vault.to_json().map_err(|e| e.to_string())?;
        let blob = crypto::encrypt(&json, &pw).map_err(|_| "encryption failed".to_string())?;
        let path = storage::save_encrypted_backup(&blob).map_err(|e| e.to_string())?;
        Ok(path.display().to_string())
    };

    let create_validated_backup = move || -> Result<String, String> {
        let Some(pw) = master_password() else {
            return Err("Vault is locked.".into());
        };
        let Some(current_vault) = vault() else {
            return Err("Vault is unavailable.".into());
        };
        let json = current_vault.to_json().map_err(|e| e.to_string())?;
        let blob = crypto::encrypt(&json, &pw).map_err(|_| "encryption failed".to_string())?;
        let restored = crypto::decrypt(&blob, &pw).map_err(|_| {
            "backup validation failed: encrypted data could not be decrypted".to_string()
        })?;
        let restored_vault = Vault::from_json(&restored)
            .map_err(|_| "backup validation failed: vault data could not be parsed".to_string())?;
        if restored_vault.to_json().map_err(|e| e.to_string())? != json {
            return Err("backup validation failed: restored data does not match the vault".into());
        }
        let path = storage::save_encrypted_backup(&blob).map_err(|e| e.to_string())?;
        Ok(path.display().to_string())
    };

    let mut restore_from_backup = move || -> Result<Option<String>, String> {
        let Some(path) = storage::choose_backup_file().map_err(|e| e.to_string())? else {
            return Ok(None);
        };
        let Some(password) = master_password() else {
            return Err("Restore failed: vault is locked.".into());
        };
        let blob = storage::load_encrypted_backup(&path).map_err(|e| e.to_string())?;
        let restored = crypto::decrypt(&blob, &password)
            .map_err(|_| "Restore failed: backup could not be decrypted.".to_string())?;
        let restored_vault = Vault::from_json(&restored)
            .map_err(|_| "Restore failed: backup vault data is invalid.".to_string())?;

        let backup_path = {
            let Some(current_vault) = vault() else {
                return Err("Restore failed: vault is unavailable.".into());
            };
            let json = current_vault.to_json().map_err(|e| e.to_string())?;
            let current_blob =
                crypto::encrypt(&json, &password).map_err(|_| "encryption failed".to_string())?;
            let validated = crypto::decrypt(&current_blob, &password)
                .map_err(|_| "Restore failed: safety backup could not be validated.".to_string())?;
            let validated_vault = Vault::from_json(&validated).map_err(|_| {
                "Restore failed: safety backup vault data could not be parsed.".to_string()
            })?;
            if validated_vault.to_json().map_err(|e| e.to_string())? != json {
                return Err(
                    "Restore failed: safety backup validation did not match the vault.".into(),
                );
            }
            storage::save_encrypted_backup(&current_blob)
                .map_err(|e| e.to_string())?
                .display()
                .to_string()
        };

        let active_id = active_vault_id();
        storage::save_encrypted_for_vault(active_id.as_deref(), &blob)
            .map_err(|e| e.to_string())?;
        vault.set(Some(restored_vault));
        Ok(Some(format!(
            "Restored backup {}; local backup: {backup_path}",
            path.display()
        )))
    };

    let mut sync_now = move || -> Result<String, String> {
        let current_settings = settings();
        if !current_settings.sync_enabled {
            return Err("Sync is disabled in Settings.".into());
        }
        let Some(folder) = current_settings.sync_folder.as_deref() else {
            return Err("Choose a local sync folder in Settings first.".into());
        };
        let Some(password) = master_password() else {
            return Err("Vault is locked.".into());
        };
        let Some(current_vault) = vault() else {
            return Err("Vault is unavailable.".into());
        };
        let json = current_vault.to_json().map_err(|e| e.to_string())?;
        let blob =
            crypto::encrypt(&json, &password).map_err(|_| "encryption failed".to_string())?;
        let remote =
            storage::download_sync(std::path::Path::new(folder)).map_err(|e| e.to_string())?;
        if let Some(remote) = remote {
            if remote.revision > current_settings.sync_revision {
                return Err(format!(
                    "Sync conflict: remote revision {} is newer than local revision {}.",
                    remote.revision, current_settings.sync_revision
                ));
            }
        }
        let revision = current_settings.sync_revision + 1;
        let envelope = SyncEnvelope::new(current_settings.sync_device_id.clone(), revision, blob);
        storage::upload_sync(std::path::Path::new(folder), &envelope).map_err(|e| e.to_string())?;
        let mut updated = current_settings;
        updated.sync_revision = revision;
        updated.last_sync_at = Some(chrono::Utc::now());
        storage::save_settings(&updated).map_err(|e| e.to_string())?;
        settings.set(updated);
        Ok(format!("Vault synced at revision {revision}"))
    };

    let mut restore_remote = move || -> Result<String, String> {
        let current_settings = settings();
        if !current_settings.sync_enabled {
            return Err("Sync is disabled in Settings.".into());
        }
        let Some(folder) = current_settings.sync_folder.as_deref() else {
            return Err("Choose a local sync folder in Settings first.".into());
        };
        let Some(password) = master_password() else {
            return Err("Vault is locked.".into());
        };
        let Some(remote) =
            storage::download_sync(std::path::Path::new(folder)).map_err(|e| e.to_string())?
        else {
            return Err("No remote vault was found in the sync folder.".into());
        };
        let restored = crypto::decrypt(&remote.vault, &password)
            .map_err(|_| "Restore failed: remote vault could not be decrypted.".to_string())?;
        let restored_vault = Vault::from_json(&restored)
            .map_err(|_| "Restore failed: remote vault data is invalid.".to_string())?;

        let backup_path = create_validated_backup()?;
        let active_id = active_vault_id();
        storage::save_encrypted_for_vault(active_id.as_deref(), &remote.vault)
            .map_err(|e| e.to_string())?;
        vault.set(Some(restored_vault));

        let mut updated = current_settings;
        updated.sync_revision = remote.revision;
        updated.last_sync_at = Some(chrono::Utc::now());
        storage::save_settings(&updated).map_err(|e| e.to_string())?;
        settings.set(updated);
        Ok(format!(
            "Restored remote revision {}; local backup: {backup_path}",
            remote.revision
        ))
    };

    let lock = move |_| {
        vault.set(None);
        master_password.set(None);
        screen.set(Screen::Unlock);
    };

    #[cfg(all(
        feature = "desktop",
        any(target_os = "windows", target_os = "linux", target_os = "macos")
    ))]
    {
        let _native_menu_handler =
            dioxus::desktop::use_muda_event_handler(move |event| match event.id().as_ref() {
                "add-entry" => {
                    editing.set(Some(VaultEntry::new(
                        String::new(),
                        String::new(),
                        String::new(),
                        String::new(),
                        String::new(),
                    )));
                    show_editor.set(true);
                }
                "new-folder" => {
                    new_folder_name.set(String::new());
                    new_folder_parent_id.set(String::new());
                    show_folder_modal.set(true);
                }
                "settings" => show_settings.set(true),
                "switch-vault" => show_vault_switcher.set(true),
                "generator" => show_generator.set(true),
                "backup" => match create_backup() {
                    Ok(path) => copy_timer.send(ToastCommand::Show(ToastState {
                        id: 0,
                        label: format!("Backup saved to {path}"),
                        duration_ms: 5000,
                        remaining_ms: 5000,
                    })),
                    Err(error) => save_error.set(Some(format!("Backup failed: {error}"))),
                },
                "restore-file" => match restore_from_backup() {
                    Ok(Some(message)) => copy_timer.send(ToastCommand::Show(ToastState {
                        id: 0,
                        label: message,
                        duration_ms: 7000,
                        remaining_ms: 7000,
                    })),
                    Ok(None) => {}
                    Err(error) => save_error.set(Some(error)),
                },
                "sync-now" => match sync_now() {
                    Ok(message) => copy_timer.send(ToastCommand::Show(ToastState {
                        id: 0,
                        label: message,
                        duration_ms: 5000,
                        remaining_ms: 5000,
                    })),
                    Err(error) => save_error.set(Some(format!("Sync failed: {error}"))),
                },
                "restore" => match restore_remote() {
                    Ok(message) => copy_timer.send(ToastCommand::Show(ToastState {
                        id: 0,
                        label: message,
                        duration_ms: 7000,
                        remaining_ms: 7000,
                    })),
                    Err(error) => save_error.set(Some(error)),
                },
                "lock" => {
                    vault.set(None);
                    master_password.set(None);
                    screen.set(Screen::Unlock);
                }
                "about" => copy_timer.send(ToastCommand::Show(ToastState {
                    id: 0,
                    label: "JPass password manager".to_string(),
                    duration_ms: 4000,
                    remaining_ms: 4000,
                })),
                _ => {}
            });
    }

    let entries: Vec<VaultEntry> = vault()
        .map(|v| v.entries)
        .unwrap_or_default()
        .into_iter()
        .filter(|e| {
            let q = search().to_lowercase();
            let matches_folder = match selected_folder() {
                Some(folder_id) => e
                    .folder_id
                    .map(|entry_folder_id| {
                        vault()
                            .as_ref()
                            .map(|current_vault| {
                                current_vault.folder_is_in_subtree(entry_folder_id, folder_id)
                            })
                            .unwrap_or(false)
                    })
                    .unwrap_or(false),
                None => true,
            };
            matches_folder
                && (q.is_empty()
                    || e.title.to_lowercase().contains(&q)
                    || e.username.to_lowercase().contains(&q)
                    || e.url.to_lowercase().contains(&q))
        })
        .collect();

    let folders: Vec<VaultFolder> = vault().map(|v| v.folders).unwrap_or_default();
    use_effect(move || {
        let folders = vault().map(|v| v.folders).unwrap_or_default();
        if !expanded_initialized() && !folders.is_empty() {
            expanded_folders.set(folders.iter().map(|folder| folder.id).collect());
            expanded_initialized.set(true);
        }

        if let Some(folder_id) = selected_folder() {
            if !folders.iter().any(|folder| folder.id == folder_id) {
                selected_folder.set(None);
                return;
            }

            let mut updated = expanded_folders();
            for ancestor in folder_ancestors(&folders, folder_id) {
                updated.insert(ancestor);
            }
            expanded_folders.set(updated);
        }
    });
    let expanded_snapshot = expanded_folders();
    let folders_for_select = folders.clone();
    let folder_tree = flatten_folder_tree(&folders);
    let folder_tree_for_select = folder_tree.clone();
    let notification_snapshot = notification();

    rsx! {
        div { class: if settings().theme == AppTheme::Light { "vault-screen light-theme" } else { "vault-screen" },
            div { class: "toolbar",
                input {
                    class: "search",
                    placeholder: "Search entries…",
                    value: "{search}",
                    oninput: move |e| search.set(e.value()),
                }
                if settings().show_primary_action_icons {
                    button {
                        class: if settings().primary_action_display == PrimaryActionDisplay::Icons { "icon-button primary-action" } else { "primary-action" },
                        title: "Add entry",
                        aria_label: "Add entry",
                        onclick: move |_| {
                            editing.set(Some(VaultEntry::new(String::new(), String::new(), String::new(), String::new(), String::new())));
                            show_editor.set(true);
                        },
                        if settings().primary_action_display == PrimaryActionDisplay::Icons { "+" } else { "+ Add Entry" }
                    }
                    button {
                        class: if settings().primary_action_display == PrimaryActionDisplay::Icons { "icon-button primary-action" } else { "primary-action" },
                        title: "New folder",
                        aria_label: "New folder",
                        onclick: move |_| {
                            new_folder_name.set(String::new());
                            new_folder_parent_id.set(String::new());
                            show_folder_modal.set(true);
                        },
                        if settings().primary_action_display == PrimaryActionDisplay::Icons { "📁+" } else { "+ New Folder" }
                    }
                    button {
                        class: if settings().primary_action_display == PrimaryActionDisplay::Icons { "secondary-btn icon-button primary-action" } else { "secondary-btn primary-action" },
                        title: "Settings",
                        aria_label: "Settings",
                        onclick: move |_| show_settings.set(true),
                        if settings().primary_action_display == PrimaryActionDisplay::Icons { "⚙" } else { "Settings" }
                    }
                    button {
                        class: if settings().primary_action_display == PrimaryActionDisplay::Icons { "secondary-btn icon-button primary-action" } else { "secondary-btn primary-action" },
                        title: "Password generator",
                        aria_label: "Password generator",
                        onclick: move |_| show_generator.set(true),
                        if settings().primary_action_display == PrimaryActionDisplay::Icons { "✦" } else { "Generator" }
                    }
                    button {
                        class: if settings().primary_action_display == PrimaryActionDisplay::Icons { "secondary-btn icon-button primary-action" } else { "secondary-btn primary-action" },
                        title: "Create backup",
                        aria_label: "Create backup",
                        onclick: move |_| match create_backup() {
                            Ok(path) => copy_timer.send(ToastCommand::Show(ToastState {
                                id: 0,
                                label: format!("Backup saved to {path}"),
                                duration_ms: 5000,
                                remaining_ms: 5000,
                            })),
                            Err(error) => save_error.set(Some(format!("Backup failed: {error}"))),
                        },
                        if settings().primary_action_display == PrimaryActionDisplay::Icons { "📤" } else { "Backup" }
                    }
                    button {
                        class: if settings().primary_action_display == PrimaryActionDisplay::Icons { "secondary-btn icon-button primary-action" } else { "secondary-btn primary-action" },
                        title: "Restore from backup file",
                        aria_label: "Restore from backup file",
                        onclick: move |_| match restore_from_backup() {
                            Ok(Some(message)) => copy_timer.send(ToastCommand::Show(ToastState {
                                id: 0,
                                label: message,
                                duration_ms: 7000,
                                remaining_ms: 7000,
                            })),
                            Ok(None) => {}
                            Err(error) => save_error.set(Some(error)),
                        },
                        if settings().primary_action_display == PrimaryActionDisplay::Icons { "↥" } else { "Restore from File" }
                    }
                    if settings().sync_enabled {
                        button {
                            class: if settings().primary_action_display == PrimaryActionDisplay::Icons { "secondary-btn icon-button primary-action sync-now-button" } else { "secondary-btn primary-action sync-now-button" },
                            title: "Sync vault",
                            aria_label: "Sync vault",
                            onclick: move |_| match sync_now() {
                                Ok(message) => copy_timer.send(ToastCommand::Show(ToastState {
                                    id: 0,
                                    label: message,
                                    duration_ms: 5000,
                                    remaining_ms: 5000,
                                })),
                                Err(error) => save_error.set(Some(format!("Sync failed: {error}"))),
                            },
                            if settings().primary_action_display == PrimaryActionDisplay::Icons { "↻" } else { "Sync Now" }
                        }
                    }
                    button {
                        class: if settings().primary_action_display == PrimaryActionDisplay::Icons { "secondary-btn icon-button primary-action" } else { "secondary-btn primary-action" },
                        title: "Download and restore vault",
                        aria_label: "Download and restore vault",
                        onclick: move |_| match restore_remote() {
                            Ok(message) => copy_timer.send(ToastCommand::Show(ToastState {
                                id: 0,
                                label: message,
                                duration_ms: 7000,
                                remaining_ms: 7000,
                            })),
                            Err(error) => save_error.set(Some(error)),
                        },
                        if settings().primary_action_display == PrimaryActionDisplay::Icons { "↓" } else { "Restore" }
                    }
                    button {
                        class: if settings().primary_action_display == PrimaryActionDisplay::Icons { "lock-btn icon-button primary-action" } else { "lock-btn primary-action" },
                        title: "Lock vault",
                        aria_label: "Lock vault",
                        onclick: lock,
                        if settings().primary_action_display == PrimaryActionDisplay::Icons { "🔒" } else { "Lock" }
                    }
                }
            }

            if let Some(msg) = save_error() {
                p { class: "error", "{msg}" }
            }

            if !notification_snapshot.is_empty() {
                div { class: match settings().toast_position {
                    ToastPosition::TopLeft => "toast-stack toast-top-left",
                    ToastPosition::TopCenter => "toast-stack toast-top-center",
                    ToastPosition::TopRight => "toast-stack toast-top-right",
                    ToastPosition::CenterLeft => "toast-stack toast-center-left",
                    ToastPosition::Center => "toast-stack toast-center",
                    ToastPosition::CenterRight => "toast-stack toast-center-right",
                    ToastPosition::BottomLeft => "toast-stack toast-bottom-left",
                    ToastPosition::BottomCenter => "toast-stack toast-bottom-center",
                    ToastPosition::BottomRight => "toast-stack toast-bottom-right",
                },
                    for toast in notification_snapshot {
                        div { class: "toast",
                            div { class: "toast-content",
                                span { "{toast.label}" }
                                div { class: "toast-timer-bar",
                                    div { class: "toast-timer-fill", style: "width: {((toast.remaining_ms as f64 / toast.duration_ms.max(1) as f64) * 100.0).clamp(0.0, 100.0)}%" }
                                }
                            }
                            button {
                                class: "toast-close",
                                onclick: {
                                    let id = toast.id;
                                    move |_| copy_timer.send(ToastCommand::Cancel(id))
                                },
                                "×"
                            }
                        }
                    }
                }
            }

            div { class: "vault-layout",
                aside { class: "folder-panel",
                    div { class: "folder-panel-actions",
                        button {
                            class: "folder-tree-action",
                            onclick: move |_| expanded_folders.set(folders.iter().map(|folder| folder.id).collect()),
                            "Expand all"
                        }
                        button {
                            class: "folder-tree-action",
                            onclick: move |_| expanded_folders.set(HashSet::new()),
                            "Collapse all"
                        }
                    }
                    button {
                        class: if selected_folder() == None { "folder-item active" } else { "folder-item" },
                        onclick: move |_| selected_folder.set(None),
                        "All entries"
                    }
                    for (folder, depth) in folder_tree {
                        if folder_is_visible(&folders, folder.id, &expanded_snapshot) {
                            div {
                                class: if depth > 0 { "folder-tree-row nested" } else { "folder-tree-row" },
                                style: "margin-left: {depth}rem",
                                oncontextmenu: {
                                    let id = folder.id;
                                    move |_| {
                                        selected_folder.set(Some(id));
                                        folder_context_menu.set(Some(id));
                                    }
                                },
                                if folder_has_children(&folders, folder.id) {
                                    button {
                                        class: "folder-expand-button",
                                        title: if expanded_snapshot.contains(&folder.id) { "Collapse folder" } else { "Expand folder" },
                                        aria_label: if expanded_snapshot.contains(&folder.id) { "Collapse folder" } else { "Expand folder" },
                                        onclick: {
                                            let id = folder.id;
                                            move |_| {
                                                let mut updated = expanded_folders();
                                                if !updated.remove(&id) {
                                                    updated.insert(id);
                                                }
                                                expanded_folders.set(updated);
                                            }
                                        },
                                        if expanded_snapshot.contains(&folder.id) { "▾" } else { "▸" }
                                    }
                                } else {
                                    span { class: "folder-expand-spacer", "" }
                                }
                                button {
                                    class: if selected_folder() == Some(folder.id) { "folder-item active" } else { "folder-item" },
                                    onclick: {
                                        let id = folder.id;
                                        move |_| selected_folder.set(Some(id))
                                    },
                                    span { "{folder.name}" }
                                }
                                if folder_context_menu() == Some(folder.id) {
                                    div { class: "folder-context-menu",
                                        button {
                                            class: "context-menu-item",
                                            onclick: {
                                                let folder_name = folder.name.clone();
                                                let folder_id = folder.id;
                                                move |_| {
                                                    rename_folder_id.set(Some(folder_id));
                                                    rename_folder_name.set(folder_name.clone());
                                                    show_folder_rename_modal.set(true);
                                                    folder_context_menu.set(None);
                                                }
                                            },
                                            "Rename"
                                        }
                                        button {
                                            class: "context-menu-item",
                                            onclick: {
                                                let folder = folder.clone();
                                                move |_| {
                                                    pending_folder_move.set(Some(folder.clone()));
                                                    folder_context_menu.set(None);
                                                }
                                            },
                                            "Move"
                                        }
                                        button {
                                            class: "context-menu-item",
                                            onclick: {
                                                let folder = folder.clone();
                                                move |_| {
                                                    pending_folder_delete.set(Some(folder.clone()));
                                                    folder_context_menu.set(None);
                                                }
                                            },
                                            "Delete"
                                        }
                                        button {
                                            class: "context-menu-item",
                                            onclick: move |_| {
                                                folder_context_menu.set(None);
                                            },
                                            "Close"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                div { class: "entry-panel",
                    ul { class: "entry-list",
                        for entry in entries {
                            li {
                                key: "{entry.id}",
                                div { class: "entry-main",
                                    span { class: "entry-title", "{entry.title}" }
                                    span { class: "entry-username", "{entry.username}" }
                                }
                                div { class: "entry-actions",
                                    button {
                                        class: if settings().entry_action_display == EntryActionDisplay::Icons { "icon-button" } else { "" },
                                        title: "Move entry",
                                        aria_label: "Move entry",
                                        onclick: {
                                            let entry = entry.clone();
                                            move |_| pending_move.set(Some(entry.clone()))
                                        },
                                        if settings().entry_action_display == EntryActionDisplay::Icons { "⇄" } else { "Move" }
                                    }
                                    button {
                                        class: if settings().entry_action_display == EntryActionDisplay::Icons { "icon-button" } else { "" },
                                        title: "Copy username",
                                        aria_label: "Copy username",
                                        onclick: {
                                            let username = entry.username.clone();
                                            let timeout = settings().clipboard_timeout_secs.max(1);
                                            move |_| {
                                                clipboard::copy_to_clipboard(&username);
                                                copy_timer.send(ToastCommand::Show(ToastState {
                                                    id: 0,
                                                    label: "Username copied".into(),
                                                    duration_ms: timeout.saturating_mul(1000),
                                                    remaining_ms: timeout.saturating_mul(1000),
                                                }));
                                            }
                                        },
                                        if settings().entry_action_display == EntryActionDisplay::Icons { "👤" } else { "Copy user" }
                                    }
                                    button {
                                        class: if settings().entry_action_display == EntryActionDisplay::Icons { "icon-button" } else { "" },
                                        title: "Copy password",
                                        aria_label: "Copy password",
                                        onclick: {
                                            let password = entry.password.clone();
                                            let timeout = settings().clipboard_timeout_secs.max(1);
                                            move |_| {
                                                clipboard::copy_to_clipboard(&password);
                                                copy_timer.send(ToastCommand::Show(ToastState {
                                                    id: 0,
                                                    label: "Password copied".into(),
                                                    duration_ms: timeout.saturating_mul(1000),
                                                    remaining_ms: timeout.saturating_mul(1000),
                                                }));
                                            }
                                        },
                                        if settings().entry_action_display == EntryActionDisplay::Icons { "⚿" } else { "Copy pass" }
                                    }
                                    button {
                                        class: if settings().entry_action_display == EntryActionDisplay::Icons { "icon-button" } else { "" },
                                        title: "Edit entry",
                                        aria_label: "Edit entry",
                                        onclick: {
                                            let entry = entry.clone();
                                            move |_| {
                                                editing.set(Some(entry.clone()));
                                                show_editor.set(true);
                                            }
                                        },
                                        if settings().entry_action_display == EntryActionDisplay::Icons { "✎" } else { "Edit" }
                                    }
                                    button {
                                        class: if settings().entry_action_display == EntryActionDisplay::Icons { "danger icon-button" } else { "danger" },
                                        title: "Delete entry",
                                        aria_label: "Delete entry",
                                        onclick: {
                                            let entry = entry.clone();
                                            move |_| {
                                                if settings().confirm_delete {
                                                    pending_delete.set(Some(entry.clone()));
                                                } else {
                                                    delete_entry(entry.id);
                                                }
                                            }
                                        },
                                        if settings().entry_action_display == EntryActionDisplay::Icons { "⌫" } else { "Delete" }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if show_editor() {
                if let Some(entry) = editing() {
                    EntryEditor {
                        entry,
                        settings,
                        generated_password: generated_entry_password,
                        on_open_generator: move |_| show_entry_generator.set(true),
                        folders: folders_for_select.clone(),
                        on_cancel: move |_| show_editor.set(false),
                        on_save: move |(mut updated, new_folder_name, new_folder_parent): (VaultEntry, Option<String>, Option<Uuid>)| {
                            if let Some(mut v) = vault() {
                                if let Some(name) = new_folder_name {
                                    match v.create_folder(&name, new_folder_parent) {
                                        Ok(folder) => updated.folder_id = Some(folder.id),
                                        Err(error) => {
                                            save_error.set(Some(error));
                                            return;
                                        }
                                    }
                                }
                                if v.entries.iter().any(|e| e.id == updated.id) {
                                    v.update_entry(updated);
                                } else {
                                    v.add_entry(updated);
                                }
                                match persist(&v) {
                                    Ok(()) => {
                                        vault.set(Some(v));
                                        save_error.set(None);
                                        show_editor.set(false);
                                    }
                                    Err(e) => save_error.set(Some(e)),
                                }
                            }
                        },
                    }
                }
            }

            if show_folder_modal() {
                div { class: "modal-backdrop",
                    div { class: "modal",
                        h2 { "New Folder" }
                        label { "Parent folder" }
                        select {
                            value: "{new_folder_parent_id}",
                            onchange: move |event| new_folder_parent_id.set(event.value()),
                            option { value: "", "Top-level folder" }
                            for (folder, _depth) in folder_tree_for_select.clone() {
                                option {
                                    value: "{folder.id}",
                                    "{folder_path(&folders, folder.id)}"
                                }
                            }
                        }
                        input {
                            placeholder: "Folder name",
                            value: "{new_folder_name}",
                            oninput: move |e| new_folder_name.set(e.value()),
                            onkeydown: move |e| {
                                if e.key() == Key::Enter {
                                    if let Some(mut v) = vault() {
                                        let name = new_folder_name();
                                        match v.create_folder(&name, Uuid::parse_str(&new_folder_parent_id()).ok()) {
                                            Ok(folder) => {
                                                match persist(&v) {
                                                    Ok(()) => {
                                                        vault.set(Some(v));
                                                        selected_folder.set(Some(folder.id));
                                                        save_error.set(None);
                                                        show_folder_modal.set(false);
                                                        new_folder_name.set(String::new());
                                                        new_folder_parent_id.set(String::new());
                                                    }
                                                    Err(err) => save_error.set(Some(err)),
                                                }
                                            }
                                            Err(err) => save_error.set(Some(err)),
                                        }
                                    }
                                }
                            },
                        }
                        div { class: "modal-actions",
                            button { onclick: move |_| { show_folder_modal.set(false); new_folder_name.set(String::new()); new_folder_parent_id.set(String::new()); }, "Cancel" }
                            button {
                                class: "primary",
                                onclick: move |_| {
                                    if let Some(mut v) = vault() {
                                        let name = new_folder_name();
                                        match v.create_folder(&name, Uuid::parse_str(&new_folder_parent_id()).ok()) {
                                            Ok(folder) => {
                                                match persist(&v) {
                                                    Ok(()) => {
                                                        vault.set(Some(v));
                                                        selected_folder.set(Some(folder.id));
                                                        save_error.set(None);
                                                        show_folder_modal.set(false);
                                                        new_folder_name.set(String::new());
                                                        new_folder_parent_id.set(String::new());
                                                    }
                                                    Err(err) => save_error.set(Some(err)),
                                                }
                                            }
                                            Err(err) => save_error.set(Some(err)),
                                        }
                                    }
                                },
                                "Create"
                            }
                        }
                    }
                }
            }

            if show_folder_rename_modal() {
                div { class: "modal-backdrop",
                    div { class: "modal",
                        h2 { "Rename Folder" }
                        label { "New folder name" }
                        input {
                            value: "{rename_folder_name}",
                            oninput: move |e| rename_folder_name.set(e.value()),
                            onkeydown: move |e| {
                                if e.key() == Key::Enter {
                                    if let Some(folder_id) = rename_folder_id() {
                                        if let Some(mut v) = vault() {
                                            let name = rename_folder_name();
                                            match v.rename_folder(folder_id, &name) {
                                                Ok(()) => match persist(&v) {
                                                    Ok(()) => {
                                                        vault.set(Some(v));
                                                        save_error.set(None);
                                                        show_folder_rename_modal.set(false);
                                                        rename_folder_id.set(None);
                                                        rename_folder_name.set(String::new());
                                                    }
                                                    Err(err) => save_error.set(Some(err)),
                                                },
                                                Err(err) => save_error.set(Some(err)),
                                            }
                                        }
                                    }
                                }
                            },
                        }
                        div { class: "modal-actions",
                            button {
                                onclick: move |_| {
                                    show_folder_rename_modal.set(false);
                                    rename_folder_id.set(None);
                                    rename_folder_name.set(String::new());
                                },
                                "Cancel"
                            }
                            button {
                                class: "primary",
                                onclick: move |_| {
                                    if let Some(folder_id) = rename_folder_id() {
                                        if let Some(mut v) = vault() {
                                            let name = rename_folder_name();
                                            match v.rename_folder(folder_id, &name) {
                                                Ok(()) => match persist(&v) {
                                                    Ok(()) => {
                                                        vault.set(Some(v));
                                                        save_error.set(None);
                                                        show_folder_rename_modal.set(false);
                                                        rename_folder_id.set(None);
                                                        rename_folder_name.set(String::new());
                                                    }
                                                    Err(err) => save_error.set(Some(err)),
                                                },
                                                Err(err) => save_error.set(Some(err)),
                                            }
                                        }
                                    }
                                },
                                "Save"
                            }
                        }
                    }
                }
            }

            if show_settings() {
                SettingsDialog {
                    settings,
                    on_close: move |_| show_settings.set(false),
                    on_error: move |message: String| save_error.set(Some(message)),
                    on_sync: move |_| match sync_now() {
                        Ok(message) => copy_timer.send(ToastCommand::Show(ToastState {
                            id: 0,
                            label: message,
                            duration_ms: 5000,
                            remaining_ms: 5000,
                        })),
                        Err(error) => save_error.set(Some(format!("Sync failed: {error}"))),
                    },
                }
            }

            if show_generator() {
                PasswordGeneratorDialog {
                    settings,
                    on_close: move |_| show_generator.set(false),
                    on_copy: move |password: String| {
                        let timeout = settings().clipboard_timeout_secs.max(1);
                        clipboard::copy_to_clipboard(&password);
                        copy_timer.send(ToastCommand::Show(ToastState {
                            id: 0,
                            label: "Generated password copied".into(),
                            duration_ms: timeout.saturating_mul(1000),
                            remaining_ms: timeout.saturating_mul(1000),
                        }));
                    },
                    on_use: move |password: String| {
                        let timeout = settings().clipboard_timeout_secs.max(1);
                        clipboard::copy_to_clipboard(&password);
                        copy_timer.send(ToastCommand::Show(ToastState {
                            id: 0,
                            label: "Generated password copied".into(),
                            duration_ms: timeout.saturating_mul(1000),
                            remaining_ms: timeout.saturating_mul(1000),
                        }));
                        show_generator.set(false);
                    },
                }
            }

            if show_entry_generator() {
                PasswordGeneratorDialog {
                    settings,
                    on_close: move |_| show_entry_generator.set(false),
                    on_copy: move |password: String| {
                        let timeout = settings().clipboard_timeout_secs.max(1);
                        clipboard::copy_to_clipboard(&password);
                        copy_timer.send(ToastCommand::Show(ToastState {
                            id: 0,
                            label: "Generated password copied".into(),
                            duration_ms: timeout.saturating_mul(1000),
                            remaining_ms: timeout.saturating_mul(1000),
                        }));
                    },
                    on_use: move |password: String| {
                        generated_entry_password.set(Some(password));
                        show_entry_generator.set(false);
                    },
                }
            }

            if let Some(entry) = pending_delete() {
                div { class: "modal-backdrop",
                    div { class: "modal confirmation-modal",
                        span { class: "eyebrow danger-eyebrow", "DESTRUCTIVE ACTION" }
                        h2 { "Delete entry?" }
                        p { "This will permanently remove ", strong { "{entry.title}" }, " from your vault." }
                        div { class: "modal-actions",
                            button {
                                class: "secondary-btn",
                                onclick: move |_| pending_delete.set(None),
                                "Cancel"
                            }
                            button {
                                class: "secondary-btn",
                                onclick: {
                                    let id = entry.id;
                                    move |_| {
                                        delete_entry(id);
                                        pending_delete.set(None);
                                    }
                                },
                                "Delete without backup"
                            }
                            button {
                                class: "primary",
                                onclick: {
                                    let id = entry.id;
                                    move |_| match create_validated_backup() {
                                        Ok(path) => {
                                            delete_entry(id);
                                            pending_delete.set(None);
                                            copy_timer.send(ToastCommand::Show(ToastState {
                                                id: 0,
                                                label: format!("Backup validated, then entry deleted: {path}"),
                                                duration_ms: 5000,
                                                remaining_ms: 5000,
                                            }));
                                        }
                                        Err(error) => save_error.set(Some(format!("Backup failed; entry was not deleted: {error}"))),
                                    }
                                },
                                "Backup & Delete"
                            }
                        }
                    }
                }
            }

            if let Some(folder) = pending_folder_move() {
                MoveFolderDialog {
                    folder,
                    folders: folders.clone(),
                    on_cancel: move |_| pending_folder_move.set(None),
                    on_move: move |(folder_id, parent_id)| {
                        move_folder(folder_id, parent_id);
                        pending_folder_move.set(None);
                    },
                }
            }

            if let Some(folder) = pending_folder_delete() {
                div { class: "modal-backdrop",
                    div { class: "modal confirmation-modal",
                        span { class: "eyebrow danger-eyebrow", "DESTRUCTIVE ACTION" }
                        h2 { "Delete folder?" }
                        p { "This will permanently remove ", strong { "{folder.name}" }, " and all nested folders and entries inside it." }
                        div { class: "modal-actions",
                            button {
                                class: "secondary-btn",
                                onclick: move |_| pending_folder_delete.set(None),
                                "Cancel"
                            }
                            button {
                                class: "secondary-btn",
                                onclick: {
                                    let id = folder.id;
                                    move |_| {
                                        delete_folder(id);
                                        pending_folder_delete.set(None);
                                    }
                                },
                                "Delete without backup"
                            }
                            button {
                                class: "primary",
                                onclick: {
                                    let id = folder.id;
                                    move |_| match create_validated_backup() {
                                        Ok(path) => {
                                            delete_folder(id);
                                            pending_folder_delete.set(None);
                                            copy_timer.send(ToastCommand::Show(ToastState {
                                                id: 0,
                                                label: format!("Backup validated, then folder deleted: {path}"),
                                                duration_ms: 5000,
                                                remaining_ms: 5000,
                                            }));
                                        }
                                        Err(error) => save_error.set(Some(format!("Backup failed; folder was not deleted: {error}"))),
                                    }
                                },
                                "Backup & Delete"
                            }
                        }
                    }
                }
            }

            if let Some(entry) = pending_move() {
                MoveEntryDialog {
                    entry,
                    folders: folders_for_select.clone(),
                    on_cancel: move |_| pending_move.set(None),
                    on_move: move |selection: (Uuid, Option<Uuid>, Option<String>)| {
                        move_entry(selection);
                        pending_move.set(None);
                    },
                }
            }

            if show_vault_switcher() {
                VaultSwitchDialog {
                    settings: settings,
                    on_close: move |_| show_vault_switcher.set(false),
                    on_select: move |vault_id: String| {
                        let mut updated = settings();
                        updated.active_vault_id = Some(vault_id.clone());
                        if storage::save_settings(&updated).is_ok() {
                            vault.set(None);
                            master_password.set(None);
                            screen.set(Screen::Unlock);
                        }
                        show_vault_switcher.set(false);
                    },
                }
            }
        }
    }
}

#[component]
fn SettingsDialog(
    settings: Signal<AppSettings>,
    on_close: EventHandler<()>,
    on_error: EventHandler<String>,
    on_sync: EventHandler<()>,
) -> Element {
    let mut settings = settings;
    let mut active_section = use_signal(|| "general");

    let mut save = move |updated: AppSettings| match storage::save_settings(&updated) {
        Ok(()) => settings.set(updated),
        Err(error) => on_error.call(format!("Failed to save settings: {error}")),
    };
    let last_sync = settings()
        .last_sync_at
        .map(|value| value.to_rfc3339())
        .unwrap_or_else(|| "Never".into());

    rsx! {
        div { class: "modal-backdrop",
            div { class: "modal settings-modal",
                div { class: "settings-heading",
                    div {
                        span { class: "eyebrow", "PREFERENCES" }
                        h2 { "Settings" }
                    }
                    button {
                        class: "modal-close",
                        onclick: move |_| on_close.call(()),
                        "×"
                    }
                }
                div { class: "settings-layout",
                    nav { class: "settings-nav", aria_label: "Settings sections",
                        button {
                            class: if active_section() == "general" { "settings-nav-item active" } else { "settings-nav-item" },
                            onclick: move |_| active_section.set("general"),
                            "General"
                        }
                        button {
                            class: if active_section() == "appearance" { "settings-nav-item active" } else { "settings-nav-item" },
                            onclick: move |_| active_section.set("appearance"),
                            "Appearance"
                        }
                        button {
                            class: if active_section() == "notifications" { "settings-nav-item active" } else { "settings-nav-item" },
                            onclick: move |_| active_section.set("notifications"),
                            "Notifications"
                        }
                        button {
                            class: if active_section() == "passwords" { "settings-nav-item active" } else { "settings-nav-item" },
                            onclick: move |_| active_section.set("passwords"),
                            "Passwords"
                        }
                        button {
                            class: if active_section() == "sync" { "settings-nav-item active" } else { "settings-nav-item" },
                            onclick: move |_| active_section.set("sync"),
                            "Sync"
                        }
                    }
                    div { class: "settings-content",
                div { class: if active_section() == "appearance" { "settings-section active" } else { "settings-section" },
                    label { "Appearance" }
                    select {
                        value: match settings().theme {
                            AppTheme::Dark => "dark",
                            AppTheme::Light => "light",
                            AppTheme::System => "system",
                        },
                        onchange: move |event| {
                            let theme = match event.value().as_str() {
                                "light" => AppTheme::Light,
                                "system" => AppTheme::System,
                                _ => AppTheme::Dark,
                            };
                            let mut updated = settings();
                            updated.theme = theme;
                            save(updated);
                        },
                        option { value: "dark", "Dark" }
                        option { value: "light", "Light" }
                        option { value: "system", "Use system preference" }
                    }
                }
                div { class: if active_section() == "general" { "settings-section settings-toggle active" } else { "settings-section settings-toggle" },
                    div {
                        label { "Confirm before deleting" }
                        p { class: "settings-help", "Ask for confirmation before permanently removing an entry." }
                    }
                    input {
                        r#type: "checkbox",
                        checked: settings().confirm_delete,
                        onchange: move |event| {
                            let mut updated = settings();
                            updated.confirm_delete = event.value() == "true";
                            save(updated);
                        },
                    }
                }
                div { class: if active_section() == "appearance" { "settings-section active" } else { "settings-section" },
                    label { "Entry actions" }
                    p { class: "settings-help", "Choose descriptive buttons or compact icons in each entry row." }
                    select {
                        value: match settings().entry_action_display {
                            EntryActionDisplay::Text => "text",
                            EntryActionDisplay::Icons => "icons",
                        },
                        onchange: move |event| {
                            let mut updated = settings();
                            updated.entry_action_display = if event.value() == "icons" {
                                EntryActionDisplay::Icons
                            } else {
                                EntryActionDisplay::Text
                            };
                            save(updated);
                        },
                        option { value: "text", "Text buttons" }
                        option { value: "icons", "Compact icons" }
                    }
                }
                div { class: if active_section() == "appearance" { "settings-section active" } else { "settings-section" },
                    label { "Primary actions" }
                    p { class: "settings-help", "Choose descriptive buttons or compact icons for the main toolbar actions." }
                    select {
                        value: match settings().primary_action_display {
                            PrimaryActionDisplay::Text => "text",
                            PrimaryActionDisplay::Icons => "icons",
                        },
                        onchange: move |event| {
                            let mut updated = settings();
                            updated.primary_action_display = if event.value() == "icons" {
                                PrimaryActionDisplay::Icons
                            } else {
                                PrimaryActionDisplay::Text
                            };
                            save(updated);
                        },
                        option { value: "text", "Text buttons" }
                        option { value: "icons", "Compact icons" }
                    }
                }
                div { class: if active_section() == "appearance" { "settings-section settings-toggle active" } else { "settings-section settings-toggle" },
                    div {
                        label { "Show primary action icons" }
                        p { class: "settings-help", "When disabled, primary action buttons are hidden from the toolbar." }
                    }
                    input {
                        r#type: "checkbox",
                        checked: settings().show_primary_action_icons,
                        onchange: move |event| {
                            let mut updated = settings();
                            updated.show_primary_action_icons = event.value() == "true";
                            save(updated);
                        },
                    }
                }
                div { class: if active_section() == "general" { "settings-section active" } else { "settings-section" },
                    label { "Clipboard timeout" }
                    p { class: "settings-help", "Copied credentials are cleared automatically after this time." }
                    select {
                        value: "{settings().clipboard_timeout_secs.max(1)}",
                        onchange: move |event| {
                            if let Ok(timeout) = event.value().parse::<u64>() {
                                let mut updated = settings();
                                updated.clipboard_timeout_secs = timeout.max(1);
                                save(updated);
                            }
                        },
                        option { value: "5", "5 seconds" }
                        option { value: "10", "10 seconds" }
                        option { value: "20", "20 seconds" }
                        option { value: "60", "60 seconds" }
                    }
                }
                div { class: if active_section() == "notifications" { "settings-section active" } else { "settings-section" },
                    label { "Toast position" }
                    p { class: "settings-help", "Choose where copy and backup notifications appear." }
                    select {
                        value: match settings().toast_position {
                            ToastPosition::TopLeft => "top-left",
                            ToastPosition::TopCenter => "top-center",
                            ToastPosition::TopRight => "top-right",
                            ToastPosition::CenterLeft => "center-left",
                            ToastPosition::Center => "center",
                            ToastPosition::CenterRight => "center-right",
                            ToastPosition::BottomLeft => "bottom-left",
                            ToastPosition::BottomCenter => "bottom-center",
                            ToastPosition::BottomRight => "bottom-right",
                        },
                        onchange: move |event| {
                            let mut updated = settings();
                            updated.toast_position = match event.value().as_str() {
                                "top-left" => ToastPosition::TopLeft,
                                "top-center" => ToastPosition::TopCenter,
                                "top-right" => ToastPosition::TopRight,
                                "center-left" => ToastPosition::CenterLeft,
                                "center" => ToastPosition::Center,
                                "center-right" => ToastPosition::CenterRight,
                                "bottom-left" => ToastPosition::BottomLeft,
                                "bottom-center" => ToastPosition::BottomCenter,
                                _ => ToastPosition::BottomRight,
                            };
                            save(updated);
                        },
                        option { value: "top-left", "Top left" }
                        option { value: "top-center", "Top center" }
                        option { value: "top-right", "Top right" }
                        option { value: "center-left", "Center left" }
                        option { value: "center", "Center" }
                        option { value: "center-right", "Center right" }
                        option { value: "bottom-left", "Bottom left" }
                        option { value: "bottom-center", "Bottom center" }
                        option { value: "bottom-right", "Bottom right" }
                    }
                }
                div { class: if active_section() == "passwords" { "settings-section active" } else { "settings-section" },
                    label { "Password generator defaults" }
                    p { class: "settings-help", "These defaults apply to the standalone generator and Edit Entry." }
                    label { "Default length" }
                    input {
                        r#type: "number",
                        min: "4",
                        max: "128",
                        value: "{settings().generator_length}",
                        onchange: move |event| {
                            if let Ok(length) = event.value().parse::<usize>() {
                                let mut updated = settings();
                                updated.generator_length = length.clamp(4, 128);
                                save(updated);
                            }
                        },
                    }
                    div { class: "generator-options",
                        label { input { r#type: "checkbox", checked: settings().generator_lowercase, onchange: move |event| { let mut updated = settings(); updated.generator_lowercase = event.value() == "true"; save(updated); } } " Lowercase" }
                        label { input { r#type: "checkbox", checked: settings().generator_uppercase, onchange: move |event| { let mut updated = settings(); updated.generator_uppercase = event.value() == "true"; save(updated); } } " Uppercase" }
                        label { input { r#type: "checkbox", checked: settings().generator_digits, onchange: move |event| { let mut updated = settings(); updated.generator_digits = event.value() == "true"; save(updated); } } " Numbers" }
                        label { input { r#type: "checkbox", checked: settings().generator_symbols, onchange: move |event| { let mut updated = settings(); updated.generator_symbols = event.value() == "true"; save(updated); } } " Special characters" }
                    }
                }
                div { class: if active_section() == "passwords" { "settings-section active" } else { "settings-section" },
                    label { "Edit Entry password generation" }
                    p { class: "settings-help", "Choose direct generation or open the full generator when editing an entry." }
                    select {
                        value: match settings().edit_password_generation_mode {
                            EditPasswordGenerationMode::AutoGenerate => "auto",
                            EditPasswordGenerationMode::FullGenerator => "full",
                        },
                        onchange: move |event| {
                            let mut updated = settings();
                            updated.edit_password_generation_mode = if event.value() == "full" {
                                EditPasswordGenerationMode::FullGenerator
                            } else {
                                EditPasswordGenerationMode::AutoGenerate
                            };
                            save(updated);
                        },
                        option { value: "auto", "Auto-generate directly" }
                        option { value: "full", "Open full password generator" }
                    }
                }
                div { class: if active_section() == "sync" { "settings-section active" } else { "settings-section" },
                    label { "Vault synchronization" }
                    p { class: "settings-help", "Sync stays off until you enable it and choose a local folder." }
                    div { class: "settings-toggle",
                        div {
                            label { "Enable sync" }
                            p { class: "settings-help", "Only encrypted vault data is written to the sync folder." }
                        }
                        input {
                            r#type: "checkbox",
                            checked: settings().sync_enabled,
                            onchange: move |event| {
                                let mut updated = settings();
                                updated.sync_enabled = event.value() == "true";
                                save(updated);
                            },
                        }
                    }
                    label { "Local sync folder" }
                    div { class: "sync-folder-picker",
                        div { class: "sync-folder-path",
                            if let Some(path) = settings().sync_folder.clone() {
                                "{path}"
                            } else {
                                "No folder selected"
                            }
                        }
                        button {
                            class: "secondary-btn",
                            onclick: move |_| match storage::choose_sync_folder() {
                                Ok(Some(path)) => {
                                    let mut updated = settings();
                                    updated.sync_folder = Some(path.display().to_string());
                                    save(updated);
                                }
                                Ok(None) => {}
                                Err(error) => on_error.call(format!("Folder picker failed: {error}")),
                            },
                            "Browse..."
                        }
                    }
                    div { class: "sync-status",
                        span { "Revision: {settings().sync_revision}" }
                        span { "Device: {settings().sync_device_id}" }
                        span { "Last sync: {last_sync}" }
                    }
                    button {
                        class: "primary sync-now-button",
                        title: "Sync vault",
                        aria_label: "Sync vault",
                        disabled: !settings().sync_enabled,
                        onclick: move |_| on_sync.call(()),
                        "Sync Now"
                    }
                }
                    }
                }
                div { class: "modal-actions",
                    button { class: "primary", onclick: move |_| on_close.call(()), "Done" }
                }
            }
        }
    }
}

#[component]
fn VaultSwitchDialog(
    settings: Signal<AppSettings>,
    on_close: EventHandler<()>,
    on_select: EventHandler<String>,
) -> Element {
    let profiles = settings().vaults.clone();

    rsx! {
        div { class: "modal-backdrop",
            div { class: "modal settings-modal",
                div { class: "settings-heading",
                    div {
                        span { class: "eyebrow", "VAULTS" }
                        h2 { "Switch vault" }
                    }
                    button {
                        class: "modal-close",
                        onclick: move |_| on_close.call(()),
                        "×"
                    }
                }
                div { class: "vault-picker-list",
                    for profile in profiles {
                        button {
                            class: "secondary-btn vault-option",
                            onclick: move |_| on_select.call(profile.id.clone()),
                            "{profile.name}"
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn PasswordGeneratorDialog(
    settings: Signal<AppSettings>,
    on_close: EventHandler<()>,
    on_copy: EventHandler<String>,
    on_use: EventHandler<String>,
) -> Element {
    let mut length = use_signal(|| settings().generator_length.to_string());
    let mut lowercase = use_signal(|| settings().generator_lowercase);
    let mut uppercase = use_signal(|| settings().generator_uppercase);
    let mut digits = use_signal(|| settings().generator_digits);
    let mut symbols = use_signal(|| settings().generator_symbols);
    let mut generated = use_signal(|| {
        generate_password(PasswordOptions {
            length: settings().generator_length,
            lowercase: settings().generator_lowercase,
            uppercase: settings().generator_uppercase,
            digits: settings().generator_digits,
            symbols: settings().generator_symbols,
        })
    });

    let mut generate = move || {
        let length = length().parse::<usize>().unwrap_or(20).clamp(4, 128);
        generated.set(generate_password(PasswordOptions {
            length,
            lowercase: lowercase(),
            uppercase: uppercase(),
            digits: digits(),
            symbols: symbols(),
        }));
    };

    rsx! {
        div { class: "modal-backdrop",
            div { class: "modal generator-modal",
                div { class: "settings-heading",
                    div {
                        span { class: "eyebrow", "UTILITY" }
                        h2 { "Password generator" }
                    }
                    button { class: "modal-close", onclick: move |_| on_close.call(()), "×" }
                }
                label { "Password length" }
                input {
                    r#type: "number",
                    min: "4",
                    max: "128",
                    value: "{length}",
                    oninput: move |event| length.set(event.value()),
                }
                div { class: "generator-options",
                    label { input { r#type: "checkbox", checked: lowercase(), onchange: move |event| lowercase.set(event.value() == "true") } " Lowercase" }
                    label { input { r#type: "checkbox", checked: uppercase(), onchange: move |event| uppercase.set(event.value() == "true") } " Uppercase" }
                    label { input { r#type: "checkbox", checked: digits(), onchange: move |event| digits.set(event.value() == "true") } " Numbers" }
                    label { input { r#type: "checkbox", checked: symbols(), onchange: move |event| symbols.set(event.value() == "true") } " Special characters" }
                }
                div { class: "generated-password", aria_label: "Generated password", "{generated}" }
                div { class: "modal-actions",
                    button { class: "secondary-btn", onclick: move |_| generate(), "Generate" }
                    button { class: "primary", onclick: move |_| on_copy.call(generated()), "Copy" }
                    button { class: "primary", onclick: move |_| on_use.call(generated()), "Use password" }
                    button { class: "primary", onclick: move |_| on_close.call(()), "Done" }
                }
            }
        }
    }
}

#[component]
fn MoveFolderDialog(
    folder: VaultFolder,
    folders: Vec<VaultFolder>,
    on_cancel: EventHandler<()>,
    on_move: EventHandler<(Uuid, Option<Uuid>)>,
) -> Element {
    let mut parent_id = use_signal(|| {
        folder
            .parent_id
            .map(|id| id.to_string())
            .unwrap_or_default()
    });
    let folder_tree = flatten_folder_tree(&folders);

    let valid_destinations = folder_tree
        .iter()
        .filter(|(candidate, _)| candidate.id != folder.id)
        .filter(|(candidate, _)| {
            !folder_path(&folders, candidate.id).starts_with(&format!("{} / ", folder.name))
        })
        .collect::<Vec<_>>();

    rsx! {
        div { class: "modal-backdrop",
            div { class: "modal move-modal",
                span { class: "eyebrow", "ORGANIZE FOLDER" }
                h2 { "Move folder" }
                p { class: "settings-help", "Choose a new parent for ", strong { "{folder.name}" }, "." }
                label { "Parent folder" }
                select {
                    value: "{parent_id}",
                    onchange: move |event| parent_id.set(event.value()),
                    option { value: "", "Top-level folder" }
                    for (candidate, _depth) in valid_destinations.clone() {
                        option { value: "{candidate.id}", "{folder_path(&folders, candidate.id)}" }
                    }
                }
                div { class: "modal-actions",
                    button {
                        class: "secondary-btn",
                        onclick: move |_| on_cancel.call(()),
                        "Cancel"
                    }
                    button {
                        class: "primary",
                        onclick: {
                            let folder_id = folder.id;
                            move |_| {
                                on_move.call((folder_id, Uuid::parse_str(&parent_id()).ok()));
                            }
                        },
                        "Move folder"
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn folder_ancestors_include_parents_in_order() {
        let root = VaultFolder::new_in_parent("Root".to_string(), None);
        let child = VaultFolder::new_in_parent("Child".to_string(), Some(root.id));
        let grandchild = VaultFolder::new_in_parent("Grandchild".to_string(), Some(child.id));
        let folders = vec![root.clone(), child.clone(), grandchild.clone()];

        let ancestors = folder_ancestors(&folders, grandchild.id);
        assert_eq!(ancestors, vec![child.id, root.id]);
    }
}

#[component]
fn MoveEntryDialog(
    entry: VaultEntry,
    folders: Vec<VaultFolder>,
    on_cancel: EventHandler<()>,
    on_move: EventHandler<(Uuid, Option<Uuid>, Option<String>)>,
) -> Element {
    let mut folder_id = use_signal(|| entry.folder_id.map(|id| id.to_string()).unwrap_or_default());
    let mut new_folder_name = use_signal(String::new);
    let mut show_new_folder = use_signal(|| false);
    let folder_tree = flatten_folder_tree(&folders);

    rsx! {
        div { class: "modal-backdrop",
            div { class: "modal move-modal",
                span { class: "eyebrow", "ORGANIZE ENTRY" }
                h2 { "Move entry" }
                p { class: "settings-help", "Choose a destination for ", strong { "{entry.title}" }, "." }
                label { "Folder" }
                div { class: "folder-select-row",
                    select {
                        value: "{folder_id}",
                        onchange: move |event| folder_id.set(event.value()),
                        option { value: "", "Unfiled" }
                        for (folder, _depth) in folder_tree {
                            option { value: "{folder.id}", "{folder_path(&folders, folder.id)}" }
                        }
                    }
                    button {
                        class: "icon-button folder-create-button",
                        title: "Create folder",
                        aria_label: "Create folder",
                        onclick: move |_| show_new_folder.set(!show_new_folder()),
                        "📁+"
                    }
                }
                if show_new_folder() {
                    input {
                        placeholder: "New folder name",
                        value: "{new_folder_name}",
                        oninput: move |event| new_folder_name.set(event.value()),
                    }
                }
                div { class: "modal-actions",
                    button {
                        class: "secondary-btn",
                        onclick: move |_| on_cancel.call(()),
                        "Cancel"
                    }
                    button {
                        class: "primary",
                        onclick: {
                            let entry_id = entry.id;
                            move |_| {
                                let name = new_folder_name().trim().to_string();
                                on_move.call((
                                    entry_id,
                                    Uuid::parse_str(&folder_id()).ok(),
                                    if name.is_empty() { None } else { Some(name) },
                                ));
                            }
                        },
                        "Move entry"
                    }
                }
            }
        }
    }
}

#[component]
fn EntryEditor(
    entry: VaultEntry,
    settings: Signal<AppSettings>,
    generated_password: Signal<Option<String>>,
    folders: Vec<VaultFolder>,
    on_save: EventHandler<(VaultEntry, Option<String>, Option<Uuid>)>,
    on_cancel: EventHandler<()>,
    on_open_generator: EventHandler<()>,
) -> Element {
    let mut title = use_signal(|| entry.title.clone());
    let mut username = use_signal(|| entry.username.clone());
    let mut password = use_signal(|| entry.password.clone());
    let mut url = use_signal(|| entry.url.clone());
    let mut notes = use_signal(|| entry.notes.clone());
    let mut reveal = use_signal(|| false);
    let mut folder_id = use_signal(|| entry.folder_id.map(|id| id.to_string()).unwrap_or_default());
    let mut new_folder_name = use_signal(String::new);
    let mut show_new_folder = use_signal(|| false);
    let folder_tree = flatten_folder_tree(&folders);

    let entry_id = entry.id;
    let created_at = entry.created_at;

    use_effect(move || {
        if let Some(value) = generated_password() {
            password.set(value);
            generated_password.set(None);
        }
    });

    rsx! {
        div { class: "modal-backdrop",
            div { class: "modal",
                h2 { "Entry" }
                label { "Title" }
                input { value: "{title}", oninput: move |e| title.set(e.value()) }
                label { "Username" }
                input { value: "{username}", oninput: move |e| username.set(e.value()) }
                label { "Password" }
                div { class: "password-row",
                    input {
                        r#type: if reveal() { "text" } else { "password" },
                        value: "{password}",
                        oninput: move |e| password.set(e.value()),
                    }
                    button { onclick: move |_| reveal.set(!reveal()), if reveal() { "Hide" } else { "Show" } }
                    button {
                        onclick: move |_| {
                            if settings().edit_password_generation_mode == EditPasswordGenerationMode::FullGenerator {
                                on_open_generator.call(());
                            } else {
                                password.set(generate_password(PasswordOptions {
                                    length: settings().generator_length.clamp(4, 128),
                                    lowercase: settings().generator_lowercase,
                                    uppercase: settings().generator_uppercase,
                                    digits: settings().generator_digits,
                                    symbols: settings().generator_symbols,
                                }));
                            }
                        },
                        "Generate"
                    }
                }
                label { "URL" }
                input { value: "{url}", oninput: move |e| url.set(e.value()) }
                label { "Notes" }
                textarea { value: "{notes}", oninput: move |e| notes.set(e.value()) }
                label { "Folder" }
                div { class: "folder-select-row",
                    select {
                        value: "{folder_id}",
                        onchange: move |e| folder_id.set(e.value()),
                        option { value: "", "Unfiled" }
                        for (folder, _depth) in folder_tree {
                            option { value: "{folder.id}", "{folder_path(&folders, folder.id)}" }
                        }
                    }
                    button {
                        class: "icon-button folder-create-button",
                        title: "Create folder",
                        aria_label: "Create folder",
                        onclick: move |_| show_new_folder.set(!show_new_folder()),
                        "📁+"
                    }
                }
                if show_new_folder() {
                    input {
                        placeholder: "New folder name",
                        value: "{new_folder_name}",
                        oninput: move |e| new_folder_name.set(e.value()),
                    }
                }

                div { class: "modal-actions",
                    button { onclick: move |_| on_cancel.call(()), "Cancel" }
                    button {
                        class: "primary",
                        onclick: move |_| {
                            let selected_folder = Uuid::parse_str(&folder_id()).ok();
                            let new_folder = new_folder_name().trim().to_string();
                            on_save.call((VaultEntry {
                                id: entry_id,
                                title: title(),
                                username: username(),
                                password: password(),
                                url: url(),
                                notes: notes(),
                                folder_id: selected_folder,
                                created_at,
                                updated_at: chrono::Utc::now(),
                            }, if new_folder.is_empty() { None } else { Some(new_folder) }, selected_folder));
                        },
                        "Save"
                    }
                }
            }
        }
    }
}
