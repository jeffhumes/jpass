use crate::crypto::{self, EncryptedBlob};
use crate::model::{Vault, VaultEntry, VaultFolder};
use crate::password_gen::{generate_password, PasswordOptions};
use crate::{clipboard, storage};
use dioxus::prelude::*;
use jpass_core::{
    AppSettings, AppTheme, EditPasswordGenerationMode, EntryActionDisplay, PrimaryActionDisplay,
    SyncEnvelope, ToastPosition,
};
use uuid::Uuid;

const MAIN_CSS: &str = include_str!("../assets/main.css");

#[derive(Clone, Copy, PartialEq)]
enum Screen {
    Loading,
    CreateMaster,
    Unlock,
    Vault,
}

#[derive(Clone, Debug)]
struct ToastState {
    id: u64,
    label: String,
    duration_ms: u64,
    remaining_ms: u64,
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

    // Decide which screen to show based on whether a vault already exists on disk.
    use_effect(move || {
        if matches!(screen(), Screen::Loading) {
            match storage::load_encrypted() {
                Ok(Some(_)) => screen.set(Screen::Unlock),
                Ok(None) => screen.set(Screen::CreateMaster),
                Err(e) => {
                    error.set(Some(format!("Storage error: {e}")));
                    screen.set(Screen::CreateMaster);
                }
            }
        }
    });

    rsx! {
        document::Title { "JPass" }
        style { {MAIN_CSS} }
        div { class: "app",
            match screen() {
                Screen::Loading => rsx! { p { "Loading…" } },
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
fn CreateMasterScreen(
    screen: Signal<Screen>,
    vault: Signal<Option<Vault>>,
    master_password: Signal<Option<String>>,
    error: Signal<Option<String>>,
) -> Element {
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
        let new_vault = Vault::default();
        let json = new_vault.to_json().expect("vault serializes");
        match crypto::encrypt(&json, &pw) {
            Ok(blob) => {
                if let Err(e) = storage::save_encrypted(&blob) {
                    error.set(Some(format!("Failed to save vault: {e}")));
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
            p { "Create a master password to protect your new vault." }
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
        let blob: EncryptedBlob = match storage::load_encrypted() {
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
    let mut editing = use_signal::<Option<VaultEntry>>(|| None);
    let mut show_editor = use_signal(|| false);
    let mut show_folder_modal = use_signal(|| false);
    let mut show_settings = use_signal(|| false);
    let mut show_generator = use_signal(|| false);
    let mut show_entry_generator = use_signal(|| false);
    let mut generated_entry_password = use_signal::<Option<String>>(|| None);
    let mut pending_delete = use_signal::<Option<VaultEntry>>(|| None);
    let mut pending_move = use_signal::<Option<VaultEntry>>(|| None);
    let mut new_folder_name = use_signal(String::new);
    let mut save_error = use_signal::<Option<String>>(|| None);
    let mut notification = use_signal::<Vec<ToastState>>(Vec::new);
    let mut settings = use_signal(|| storage::load_settings().unwrap_or_default());

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
        storage::save_encrypted(&blob).map_err(|e| e.to_string())
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

    let mut move_entry =
        move |(entry_id, folder_id, new_folder_name): (Uuid, Option<Uuid>, Option<String>)| {
            if let Some(mut v) = vault() {
                let target_folder = if let Some(name) = new_folder_name {
                    match v.create_folder(&name) {
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
        let blob = crypto::encrypt(&json, &password).map_err(|_| "encryption failed".to_string())?;
        let remote = storage::download_sync(std::path::Path::new(folder)).map_err(|e| e.to_string())?;
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
        storage::upload_sync(std::path::Path::new(folder), &envelope)
            .map_err(|e| e.to_string())?;
        let mut updated = current_settings;
        updated.sync_revision = revision;
        updated.last_sync_at = Some(chrono::Utc::now());
        storage::save_settings(&updated).map_err(|e| e.to_string())?;
        settings.set(updated);
        Ok(format!("Vault synced at revision {revision}"))
    };

    let lock = move |_| {
        vault.set(None);
        master_password.set(None);
        screen.set(Screen::Unlock);
    };

    let entries: Vec<VaultEntry> = vault()
        .map(|v| v.entries)
        .unwrap_or_default()
        .into_iter()
        .filter(|e| {
            let q = search().to_lowercase();
            let matches_folder = match selected_folder() {
                Some(folder_id) => e.folder_id == Some(folder_id),
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
    let folders_for_select = folders.clone();
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
                    if settings().primary_action_display == PrimaryActionDisplay::Icons { "↻" } else { "Sync" }
                }
                button {
                    class: if settings().primary_action_display == PrimaryActionDisplay::Icons { "lock-btn icon-button primary-action" } else { "lock-btn primary-action" },
                    title: "Lock vault",
                    aria_label: "Lock vault",
                    onclick: lock,
                    if settings().primary_action_display == PrimaryActionDisplay::Icons { "🔒" } else { "Lock" }
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
                    button {
                        class: if selected_folder() == None { "folder-item active" } else { "folder-item" },
                        onclick: move |_| selected_folder.set(None),
                        "All entries"
                    }
                    for folder in folders {
                        button {
                            class: if selected_folder() == Some(folder.id) { "folder-item active" } else { "folder-item" },
                            onclick: {
                                let id = folder.id;
                                move |_| selected_folder.set(Some(id))
                            },
                            "{folder.name}"
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
                        on_save: move |(mut updated, new_folder_name): (VaultEntry, Option<String>)| {
                            if let Some(mut v) = vault() {
                                if let Some(name) = new_folder_name {
                                    match v.create_folder(&name) {
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
                        input {
                            placeholder: "Folder name",
                            value: "{new_folder_name}",
                            oninput: move |e| new_folder_name.set(e.value()),
                            onkeydown: move |e| {
                                if e.key() == Key::Enter {
                                    if let Some(mut v) = vault() {
                                        let name = new_folder_name();
                                        match v.create_folder(&name) {
                                            Ok(folder) => {
                                                match persist(&v) {
                                                    Ok(()) => {
                                                        vault.set(Some(v));
                                                        selected_folder.set(Some(folder.id));
                                                        save_error.set(None);
                                                        show_folder_modal.set(false);
                                                        new_folder_name.set(String::new());
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
                            button { onclick: move |_| { show_folder_modal.set(false); new_folder_name.set(String::new()); }, "Cancel" }
                            button {
                                class: "primary",
                                onclick: move |_| {
                                    if let Some(mut v) = vault() {
                                        let name = new_folder_name();
                                        match v.create_folder(&name) {
                                            Ok(folder) => {
                                                match persist(&v) {
                                                    Ok(()) => {
                                                        vault.set(Some(v));
                                                        selected_folder.set(Some(folder.id));
                                                        save_error.set(None);
                                                        show_folder_modal.set(false);
                                                        new_folder_name.set(String::new());
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

            if show_settings() {
                SettingsDialog {
                    settings,
                    on_close: move |_| show_settings.set(false),
                    on_error: move |message: String| save_error.set(Some(message)),
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
        }
    }
}

#[component]
fn SettingsDialog(
    settings: Signal<AppSettings>,
    on_close: EventHandler<()>,
    on_error: EventHandler<String>,
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
                    div { class: "settings-section settings-toggle",
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
                    input {
                        placeholder: "Folder path",
                        value: settings().sync_folder.clone().unwrap_or_default(),
                        oninput: move |event| {
                            let mut updated = settings();
                            let path = event.value().trim().to_string();
                            updated.sync_folder = if path.is_empty() { None } else { Some(path) };
                            save(updated);
                        },
                    }
                    div { class: "sync-status",
                        span { "Revision: {settings().sync_revision}" }
                        span { "Device: {settings().sync_device_id}" }
                        span { "Last sync: {last_sync}" }
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
fn MoveEntryDialog(
    entry: VaultEntry,
    folders: Vec<VaultFolder>,
    on_cancel: EventHandler<()>,
    on_move: EventHandler<(Uuid, Option<Uuid>, Option<String>)>,
) -> Element {
    let mut folder_id = use_signal(|| entry.folder_id.map(|id| id.to_string()).unwrap_or_default());
    let mut new_folder_name = use_signal(String::new);
    let mut show_new_folder = use_signal(|| false);

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
                        for folder in folders {
                            option { value: "{folder.id}", "{folder.name}" }
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
    on_save: EventHandler<(VaultEntry, Option<String>)>,
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
                        for folder in folders {
                            option { value: "{folder.id}", "{folder.name}" }
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
                            }, if new_folder.is_empty() { None } else { Some(new_folder) }));
                        },
                        "Save"
                    }
                }
            }
        }
    }
}
