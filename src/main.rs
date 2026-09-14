mod app;
mod clipboard;
mod crypto;
mod model;
mod password_gen;
mod platform;
mod storage;

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

    dioxus::launch(app::App);
}
