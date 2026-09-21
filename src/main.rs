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
        &MenuItem::with_id("delete-vault", "Delete Vault", true, None),
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

// On Linux, WebKitGTK reads several env vars during its own (pre-`main`) library
// initialization, so setting them from inside `main` is too late to take effect.
// If accelerated compositing initializes anyway, WebKitGTK has a known bug where
// pointer motion/click events stop reaching the web view (keyboard still works)
// even though hover/CSS looks fine visually. Re-exec the process once with the
// vars set beforehand so the fresh process picks them up from the very start.
#[cfg(target_os = "linux")]
fn ensure_linux_webkit_env_and_relaunch() {
    use std::os::unix::process::CommandExt;

    const REQUIRED: &[(&str, &str)] = &[
        ("WEBKIT_DISABLE_DMABUF_RENDERER", "1"),
        ("WEBKIT_DISABLE_COMPOSITING_MODE", "1"),
        ("LIBGL_ALWAYS_SOFTWARE", "1"),
        ("DIOXUS_ALWAYS_ON_TOP", "false"),
    ];

    let already_relaunched = std::env::var_os("JPASS_RELAUNCHED").is_some();
    let needs_any = REQUIRED
        .iter()
        .any(|(key, value)| std::env::var(key).as_deref() != Ok(value));

    if needs_any && !already_relaunched {
        let current_exe = match std::env::current_exe() {
            Ok(path) => path,
            Err(_) => return,
        };

        let err = std::process::Command::new(current_exe)
            .args(std::env::args_os().skip(1))
            .envs(REQUIRED.iter().copied())
            .env("JPASS_RELAUNCHED", "1")
            .exec();

        eprintln!("jpass: failed to relaunch with WebKit env vars set: {err}");
    }
}

fn main() {
    #[cfg(target_os = "linux")]
    ensure_linux_webkit_env_and_relaunch();

    #[cfg(feature = "desktop")]
    {
        let mut cfg = dioxus::desktop::Config::new();
        if std::env::var_os("JPASS_NO_MENU").is_none() {
            cfg = cfg.with_menu(native_menu());
        }
        dioxus::LaunchBuilder::desktop()
            .with_cfg(cfg)
            .launch(app::App);
    }

    #[cfg(not(feature = "desktop"))]
    dioxus::launch(app::App);
}
