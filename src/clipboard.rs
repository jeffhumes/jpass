//! Best-effort clipboard copy, implemented via the selected platform adapter.

use crate::platform::{PlatformAdapter, PlatformAdapterTrait};

pub fn copy_to_clipboard(text: &str) {
    let adapter = PlatformAdapter::current();
    match adapter.copy_text(text) {
        Ok(_) => {}
        Err(_) => {}
    }
}

pub fn clear_clipboard() {
    let adapter = PlatformAdapter::current();
    match adapter.clear() {
        Ok(_) => {}
        Err(_) => {}
    }
}
