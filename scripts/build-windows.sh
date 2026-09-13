#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

rustup target add x86_64-pc-windows-gnu || true
cargo build --release --target x86_64-pc-windows-gnu

out_dir="target/x86_64-pc-windows-gnu/release"
exe_path="$out_dir/jpass.exe"
webview2_loader="$(find "$out_dir/build" -path '*/out/x64/WebView2Loader.dll' -type f -print -quit)"

if [ ! -f "$exe_path" ]; then
  echo "Windows EXE was not produced at $exe_path" >&2
  exit 1
fi

if [ -z "$webview2_loader" ]; then
  echo "WebView2Loader.dll was not found in the Windows build output" >&2
  exit 1
fi

mkdir -p dist/windows
rm -f dist/windows/jpass.exe dist/windows/WebView2Loader.dll dist/windows/jpass-windows.zip
cp "$exe_path" dist/windows/
cp "$webview2_loader" dist/windows/WebView2Loader.dll
zip -j dist/windows/jpass-windows.zip dist/windows/jpass.exe dist/windows/WebView2Loader.dll

echo "Built: $exe_path"
echo "Package: dist/windows/jpass-windows.zip"
