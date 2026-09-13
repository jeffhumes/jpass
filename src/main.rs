mod app;
mod clipboard;
mod crypto;
mod model;
mod password_gen;
mod platform;
mod storage;

fn main() {
    dioxus::launch(app::App);
}
