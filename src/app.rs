use crate::crypto::{self, EncryptedBlob};
use crate::model::{Vault, VaultEntry, VaultFolder};
use crate::password_gen::{generate_password, PasswordOptions};
use crate::{clipboard, storage};
use dioxus::prelude::*;
use jpass_core::{AppSettings, AppTheme, EntryActionDisplay, PrimaryActionDisplay};
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
    label: String,
    duration_ms: u64,
    remaining_ms: u64,
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
    let mut pending_delete = use_signal::<Option<VaultEntry>>(|| None);
    let mut pending_move = use_signal::<Option<VaultEntry>>(|| None);
    let mut new_folder_name = use_signal(String::new);
    let mut save_error = use_signal::<Option<String>>(|| None);
    let mut notification = use_signal::<Option<ToastState>>(|| None);
    let mut settings = use_signal(|| storage::load_settings().unwrap_or_default());
    let clipboard_timeout = settings().clipboard_timeout_secs.max(1);

    let _copy_timer = use_future(move || {
        let active = notification();

        async move {
            let Some(current) = active else {
                return;
            };

            let total_ms = current.duration_ms.max(1);
            let mut remaining = current.remaining_ms.max(1);

            while remaining > 0 {
                futures_timer::Delay::new(std::time::Duration::from_millis(50)).await;
                remaining = remaining.saturating_sub(50);
                notification.set(Some(ToastState {
                    label: current.label.clone(),
                    duration_ms: total_ms,
                    remaining_ms: remaining.max(0),
                }));
            }

            notification.set(None);
            clipboard::clear_clipboard();
        }
    });

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
    let toast_progress = notification_snapshot.as_ref().map(|toast| {
        if toast.duration_ms == 0 {
            0.0
        } else {
            ((toast.remaining_ms as f64 / toast.duration_ms as f64) * 100.0).clamp(0.0, 100.0)
        }
    });

    rsx! {
        div { class: if settings().theme == AppTheme::Light { "vault-screen light-theme" } else { "vault-screen" },
            div { class: "toolbar",
                input {
                    class: "search",
                    placeholder: "Search entries…",
                    value: "{search}",
                    oninput: move |e| search.set(e.value()),
                }
                div { class: "clipboard-setting",
                    label { "Keep clipboard for " }
                    select {
                        value: "{clipboard_timeout}",
                        onchange: move |e| {
                            if let Ok(value) = e.value().parse::<u64>() {
                                let timeout = value.max(1);
                                let mut updated = settings();
                                updated.clipboard_timeout_secs = timeout;
                                if let Err(err) = storage::save_settings(&updated) {
                                    save_error.set(Some(format!("Failed to save clipboard setting: {err}")));
                                } else {
                                    settings.set(updated);
                                    save_error.set(None);
                                }
                            }
                        },
                        option { value: "5", "5s" }
                        option { value: "10", "10s" }
                        option { value: "20", "20s" }
                        option { value: "60", "60s" }
                    }
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
                    title: "Create backup",
                    aria_label: "Create backup",
                    onclick: move |_| match create_backup() {
                        Ok(path) => notification.set(Some(ToastState {
                            label: format!("Backup saved to {path}"),
                            duration_ms: 5000,
                            remaining_ms: 5000,
                        })),
                        Err(error) => save_error.set(Some(format!("Backup failed: {error}"))),
                    },
                    if settings().primary_action_display == PrimaryActionDisplay::Icons { "□↓" } else { "Backup" }
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

            if let Some(toast) = notification_snapshot {
                div { class: "toast",
                    div { class: "toast-content",
                        span { "{toast.label}" }
                        div { class: "toast-timer-bar",
                            div { class: "toast-timer-fill", style: "width: {toast_progress.unwrap_or(0.0)}%" }
                        }
                    }
                    button {
                        class: "toast-close",
                        onclick: move |_| notification.set(None),
                        "×"
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
                                                notification.set(Some(ToastState {
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
                                                notification.set(Some(ToastState {
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
                                class: "danger",
                                onclick: {
                                    let id = entry.id;
                                    move |_| {
                                        delete_entry(id);
                                        pending_delete.set(None);
                                    }
                                },
                                "Delete entry"
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

    let mut save = move |updated: AppSettings| match storage::save_settings(&updated) {
        Ok(()) => settings.set(updated),
        Err(error) => on_error.call(format!("Failed to save settings: {error}")),
    };

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
                div { class: "settings-section",
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
                div { class: "settings-section settings-toggle",
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
                div { class: "settings-section",
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
                div { class: "settings-section",
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
                div { class: "settings-section",
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
                div { class: "modal-actions",
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

    rsx! {
        div { class: "modal-backdrop",
            div { class: "modal move-modal",
                span { class: "eyebrow", "ORGANIZE ENTRY" }
                h2 { "Move entry" }
                p { class: "settings-help", "Choose a destination for ", strong { "{entry.title}" }, "." }
                label { "Folder" }
                select {
                    value: "{folder_id}",
                    onchange: move |event| folder_id.set(event.value()),
                    option { value: "", "Unfiled" }
                    for folder in folders {
                        option { value: "{folder.id}", "{folder.name}" }
                    }
                }
                input {
                    placeholder: "Or create a new folder",
                    value: "{new_folder_name}",
                    oninput: move |event| new_folder_name.set(event.value()),
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
    folders: Vec<VaultFolder>,
    on_save: EventHandler<(VaultEntry, Option<String>)>,
    on_cancel: EventHandler<()>,
) -> Element {
    let mut title = use_signal(|| entry.title.clone());
    let mut username = use_signal(|| entry.username.clone());
    let mut password = use_signal(|| entry.password.clone());
    let mut url = use_signal(|| entry.url.clone());
    let mut notes = use_signal(|| entry.notes.clone());
    let mut reveal = use_signal(|| false);
    let mut folder_id = use_signal(|| entry.folder_id.map(|id| id.to_string()).unwrap_or_default());
    let mut new_folder_name = use_signal(String::new);

    let entry_id = entry.id;
    let created_at = entry.created_at;

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
                        onclick: move |_| password.set(generate_password(PasswordOptions::default())),
                        "Generate"
                    }
                }
                label { "URL" }
                input { value: "{url}", oninput: move |e| url.set(e.value()) }
                label { "Notes" }
                textarea { value: "{notes}", oninput: move |e| notes.set(e.value()) }
                label { "Folder" }
                select {
                    value: "{folder_id}",
                    onchange: move |e| folder_id.set(e.value()),
                    option { value: "", "Unfiled" }
                    for folder in folders {
                        option { value: "{folder.id}", "{folder.name}" }
                    }
                }
                input {
                    placeholder: "Or create a new folder",
                    value: "{new_folder_name}",
                    oninput: move |e| new_folder_name.set(e.value()),
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
