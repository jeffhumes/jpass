//! Best-effort clipboard copy, implemented per platform.

#[cfg(feature = "desktop")]
pub fn copy_to_clipboard(text: &str) {
    if let Ok(mut clipboard) = arboard::Clipboard::new() {
        let _ = clipboard.set_text(text.to_string());
    }
}

#[cfg(feature = "desktop")]
pub fn clear_clipboard() {
    if let Ok(mut clipboard) = arboard::Clipboard::new() {
        let _ = clipboard.clear();
    }
}

#[cfg(all(feature = "web", not(feature = "desktop")))]
pub fn copy_to_clipboard(text: &str) {
    use wasm_bindgen_futures::JsFuture;
    let text = text.to_string();
    if let Some(window) = web_sys::window() {
        let clipboard = window.navigator().clipboard();
        wasm_bindgen_futures::spawn_local(async move {
            let _ = JsFuture::from(clipboard.write_text(&text)).await;
        });
    }
}

#[cfg(all(feature = "web", not(feature = "desktop")))]
pub fn clear_clipboard() {
    if let Some(window) = web_sys::window() {
        let clipboard = window.navigator().clipboard();
        wasm_bindgen_futures::spawn_local(async move {
            let _ = clipboard.write_text("").await;
        });
    }
}
