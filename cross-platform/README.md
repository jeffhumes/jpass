# Cross-platform architecture prototype

This workspace is a concrete blueprint for splitting the existing JPass app into a shared core and platform-specific adapters.

## Layout

```text
cross-platform/
├── Cargo.toml                # workspace definition
├── README.md                 # architecture overview
├── crates/
│   ├── jpass-core/           # shared domain + crypto logic
│   ├── jpass-platform/       # platform traits / interfaces
│   ├── jpass-desktop/        # desktop implementation
│   ├── jpass-android/        # Android implementation stub
│   └── jpass-ios/            # iOS implementation stub
└── ...
```

## Design goals

- Shared encryption and vault logic should compile everywhere.
- OS-specific functions are isolated behind traits.
- Desktop, Android, and iOS can each implement the same contract.
- The existing app can migrate to this model without rewriting the business logic.

## Architecture

- `jpass-core`: vault schema, encryption helpers, password generation, settings.
- `jpass-platform`: contracts for storage, clipboard, path resolution, and platform capabilities.
- `jpass-desktop`: SQLite-backed store, clipboard integration, native app directories.
- `jpass-android`: Android secure storage and clipboard adapters.
- `jpass-ios`: Keychain-backed storage and iOS clipboard adapters.

## Migration path

1. Promote the current vault model, crypto logic, and rules into `jpass-core`.
2. Replace the current feature-gated storage code with platform trait implementations.
3. Keep the Dioxus UI layer thin and dependent on the shared services.
4. Build each target separately with its own adapter implementation.
