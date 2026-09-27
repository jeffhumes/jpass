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

## Android

Install the Android target, Dioxus CLI, Android SDK command-line tools, and an Android NDK. Set the
SDK variables before invoking Dioxus:

```bash
rustup target add aarch64-linux-android
cargo install dioxus-cli --version 0.7.10 --locked
export ANDROID_HOME="$HOME/android/sdk"
export ANDROID_SDK_ROOT="$ANDROID_HOME"
export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/30.0.16248370"
```

Use Android Studio's SDK Manager, or `sdkmanager`, to install an Android platform, build tools, and an
NDK version compatible with the installed Dioxus CLI. Build an APK with:

```bash
dx bundle --platform android
```

The verified release bundle is written under:

```text
target/dx/jpass/debug/android/app/app/build/outputs/bundle/release/
```

For a fast Rust-only target check without producing an APK:

```bash
cargo check --no-default-features --features mobile --target aarch64-linux-android
```

Android data is stored in the application's private home directory under `.jpass`. The release build
still needs a native Android clipboard binding before copy and clear controls are available.

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
