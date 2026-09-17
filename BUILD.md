# JPass Build Guide

Run these commands from the repository root.

## Prerequisites

Install Rust and Cargo. Confirm the toolchain is available:

```bash
rustc --version
cargo --version
```

The default feature set builds the native desktop application.

## Run Locally

```bash
cargo run
```

## Debug Build

```bash
cargo build
```

Output:

```text
target/debug/jpass
```

## Release Build

```bash
cargo build --release
```

Output:

```text
target/release/jpass
```

## Tests and Checks

```bash
cargo test
cargo check
cargo fmt --check
```

## Android Feature Check

This checks the mobile configuration for the Android target without producing a complete APK:

```bash
rustup target add aarch64-linux-android
cargo check --no-default-features --features mobile --target aarch64-linux-android
```

A complete Android package also requires the Android SDK/NDK and platform-specific packaging configuration.

## Windows Release Package

The packaging script installs the GNU Windows target when needed, builds the release executable, locates `WebView2Loader.dll`, and creates a ZIP package.

```bash
./scripts/build-windows.sh
```

Outputs:

```text
target/x86_64-pc-windows-gnu/release/jpass.exe
dist/windows/jpass-windows.zip
```

The ZIP contains:

```text
jpass.exe
WebView2Loader.dll
```

To run the Windows build steps manually:

```bash
rustup target add x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu
```

The executable is written to:

```text
target/x86_64-pc-windows-gnu/release/jpass.exe
```

The executable must be distributed with `WebView2Loader.dll` on Windows.
