use crate::crypto::{self, EncryptedBlob};
use crate::model::{Vault, VaultEntry, VaultFolder};
use crate::password_gen::{generate_password, PasswordOptions};
use crate::{clipboard, storage};
use dioxus::prelude::*;
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
    let mut new_folder_name = use_signal(String::new);
    let mut save_error = use_signal::<Option<String>>(|| None);
    let mut notification = use_signal::<Option<ToastState>>(|| None);
    let mut clipboard_timeout = use_signal(|| {
        storage::load_settings()
            .map(|settings| settings.clipboard_timeout_secs.max(1))
            .unwrap_or_else(|_| storage::AppSettings::default_timeout().max(1))
    });

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
        div { class: "vault-screen",
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
                                clipboard_timeout.set(timeout);
                                if let Err(err) = storage::save_settings(&storage::AppSettings {
                                    clipboard_timeout_secs: timeout,
                                }) {
                                    save_error.set(Some(format!("Failed to save clipboard setting: {err}")));
                                } else {
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
                    onclick: move |_| {
                        editing.set(Some(VaultEntry::new(String::new(), String::new(), String::new(), String::new(), String::new())));
                        show_editor.set(true);
                    },
                    "+ Add Entry"
                }
                button {
                    onclick: move |_| {
                        new_folder_name.set(String::new());
                        show_folder_modal.set(true);
                    },
                    "+ New Folder"
                }
                button { class: "lock-btn", onclick: lock, "Lock" }
            }

            if let Some(msg) = save_error() {
                p { class: "error", "{msg}" }
            }

            if let Some(toast) = notification_snapshot {
                div { class: "toast",
                    div { class: "toast-content",
                        span { "{toast.label} copied" }
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
                                    select {
                                        value: match entry.folder_id { Some(id) => id.to_string(), None => String::new() },
                                        onchange: {
                                            let entry_id = entry.id;
                                            move |e| {
                                                let folder_value = e.value();
                                                let target = if folder_value.trim().is_empty() {
                                                    None
                                                } else {
                                                    Uuid::parse_str(&folder_value).ok()
                                                };

                                                if let Some(mut v) = vault() {
                                                    let changed = v.move_entry_to_folder(entry_id, target);
                                                    if changed {
                                                        match persist(&v) {
                                                            Ok(()) => {
                                                                vault.set(Some(v));
                                                                save_error.set(None);
                                                            }
                                                            Err(err) => save_error.set(Some(err)),
                                                        }
                                                    }
                                                }
                                            }
                                        },
                                        option { value: "", "Unfiled" }
                                        for folder in folders_for_select.clone() {
                                            option {
                                                value: "{folder.id}",
                                                selected: entry.folder_id == Some(folder.id),
                                                "{folder.name}"
                                            }
                                        }
                                    }
                                    button {
                                        onclick: {
                                            let username = entry.username.clone();
                                            let timeout = clipboard_timeout();
                                            move |_| {
                                                clipboard::copy_to_clipboard(&username);
                                                notification.set(Some(ToastState {
                                                    label: "Username".into(),
                                                    duration_ms: timeout.saturating_mul(1000),
                                                    remaining_ms: timeout.saturating_mul(1000),
                                                }));
                                            }
                                        },
                                        "Copy user"
                                    }
                                    button {
                                        onclick: {
                                            let password = entry.password.clone();
                                            let timeout = clipboard_timeout();
                                            move |_| {
                                                clipboard::copy_to_clipboard(&password);
                                                notification.set(Some(ToastState {
                                                    label: "Password".into(),
                                                    duration_ms: timeout.saturating_mul(1000),
                                                    remaining_ms: timeout.saturating_mul(1000),
                                                }));
                                            }
                                        },
                                        "Copy pass"
                                    }
                                    button {
                                        onclick: {
                                            let entry = entry.clone();
                                            move |_| {
                                                editing.set(Some(entry.clone()));
                                                show_editor.set(true);
                                            }
                                        },
                                        "Edit"
                                    }
                                    button {
                                        class: "danger",
                                        onclick: {
                                            let id = entry.id;
                                            move |_| {
                                                if let Some(mut v) = vault() {
                                                    v.remove_entry(id);
                                                    match persist(&v) {
                                                        Ok(()) => { vault.set(Some(v)); save_error.set(None); }
                                                        Err(e) => save_error.set(Some(e)),
                                                    }
                                                }
                                            }
                                        },
                                        "Delete"
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
                        on_cancel: move |_| show_editor.set(false),
                        on_save: move |updated: VaultEntry| {
                            if let Some(mut v) = vault() {
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
        }
    }
}

#[component]
fn EntryEditor(
    entry: VaultEntry,
    on_save: EventHandler<VaultEntry>,
    on_cancel: EventHandler<()>,
) -> Element {
    let mut title = use_signal(|| entry.title.clone());
    let mut username = use_signal(|| entry.username.clone());
    let mut password = use_signal(|| entry.password.clone());
    let mut url = use_signal(|| entry.url.clone());
    let mut notes = use_signal(|| entry.notes.clone());
    let mut reveal = use_signal(|| false);

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

                div { class: "modal-actions",
                    button { onclick: move |_| on_cancel.call(()), "Cancel" }
                    button {
                        class: "primary",
                        onclick: move |_| {
                            on_save.call(VaultEntry {
                                id: entry_id,
                                title: title(),
                                username: username(),
                                password: password(),
                                url: url(),
                                notes: notes(),
                                folder_id: entry.folder_id,
                                created_at,
                                updated_at: chrono::Utc::now(),
                            });
                        },
                        "Save"
                    }
                }
            }
        }
    }
}
