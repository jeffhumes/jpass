# JPass

JPass is a cross-platform password manager built with Rust and Dioxus. It stores credentials in an encrypted vault protected by a master password, with a desktop-first workflow and support for browser-based builds behind feature flags.

## Features

- Secure vault storage using AES-256-GCM encryption
- Master password unlock flow
- Password entries with title, username, password, URL, notes, and optional folder grouping
- Folder organization for vault entries
- Search/filtering within the vault
- Clipboard clear support after copying a password
- Persistent encrypted storage via SQLite on desktop and browser storage on web builds

## Tech Stack

- Rust
- Dioxus
- SQLite (desktop)
- Argon2 for key derivation
- AES-GCM for encryption
- Serde / serde_json for data serialization

## Project Structure

```text
JPass/
├── assets/
│   └── main.css
├── src/
│   ├── app.rs
│   ├── clipboard.rs
│   ├── crypto.rs
│   ├── main.rs
│   ├── model.rs
│   ├── password_gen.rs
│   └── storage.rs
├── Cargo.toml
├── Dioxus.toml
└── README.md
```

## Requirements

- Rust 1.70+ (recommended current stable toolchain)
- Cargo

## Getting Started

Clone the repository and run the app:

```bash
git clone <repository-url>
cd JPass
cargo run
```

This uses the default desktop configuration.

## Build

```bash
cargo build
```

For a release build:

```bash
cargo build --release
```

## Security Notes

- The master password is never stored on disk.
- Vault data is encrypted before persistence.
- Encryption keys are derived from the user’s master password using Argon2.
- The application stores only the encrypted blob and metadata, not plaintext credentials.

## Notes

The project is currently configured with a desktop-first default feature set. Web support exists through feature flags in the dependency configuration, but the default app experience is the native desktop app.

## Contributing

Contributions are welcome. If you want to improve the app, please open a pull request with a clear explanation of the change and verification steps.
