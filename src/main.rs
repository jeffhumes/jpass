#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod app;
mod clipboard;
mod crypto;
mod model;
mod password_gen;
mod platform;
mod storage;

#[cfg(feature = "desktop")]
fn native_menu() -> dioxus::desktop::muda::Menu {
    use dioxus::desktop::muda::{Menu, MenuItem, Submenu};

    let file = Submenu::new("File", true);
    let edit = Submenu::new("Edit / Settings", true);
    let help = Submenu::new("Help", true);

    file.append_items(&[
        &MenuItem::with_id("add-entry", "Add Entry", true, None),
        &MenuItem::with_id("new-folder", "New Folder", true, None),
        &MenuItem::with_id("backup", "Backup", true, None),
        &MenuItem::with_id("restore-file", "Restore from File", true, None),
        &MenuItem::with_id("sync-now", "Sync Now", true, None),
        &MenuItem::with_id("restore", "Restore", true, None),
        &MenuItem::with_id("lock", "Lock", true, None),
    ])
    .expect("failed to build File menu");
    edit.append_items(&[
        &MenuItem::with_id("settings", "Settings", true, None),
        &MenuItem::with_id("switch-vault", "Switch Vault", true, None),
        &MenuItem::with_id("rename-vault", "Rename Vault", true, None),
        &MenuItem::with_id("generator", "Generator", true, None),
    ])
    .expect("failed to build Edit / Settings menu");
    help.append(&MenuItem::with_id("about", "About JPass", true, None))
        .expect("failed to build Help menu");

    let menu = Menu::new();
    menu.append_items(&[&file, &edit, &help])
        .expect("failed to build application menu");
    menu
}

fn main() {
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_COMPOSITING_MODE").is_none() {
        std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
    }
    #[cfg(target_os = "linux")]
    if std::env::var_os("LIBGL_ALWAYS_SOFTWARE").is_none() {
        std::env::set_var("LIBGL_ALWAYS_SOFTWARE", "1");
    }
    #[cfg(target_os = "linux")]
    if std::env::var_os("DIOXUS_ALWAYS_ON_TOP").is_none() {
        std::env::set_var("DIOXUS_ALWAYS_ON_TOP", "false");
    }

    #[cfg(feature = "desktop")]
    {
        dioxus::LaunchBuilder::desktop()
            .with_cfg(dioxus::desktop::Config::new().with_menu(native_menu()))
            .launch(app::App);
    }

    #[cfg(not(feature = "desktop"))]
    dioxus::launch(app::App);
}
