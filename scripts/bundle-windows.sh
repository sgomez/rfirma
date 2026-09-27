#!/usr/bin/env bash
# Construye el instalador NSIS de Windows con el bundler de Tauri (ADR-0035).
set -euo pipefail

root="$1"
tauri="$root/rfirma-app/src-tauri"
runtime="$tauri/target/windows-runtime"

vswhere="/c/Program Files (x86)/Microsoft Visual Studio/Installer/vswhere.exe"
vs="$(cygpath -u "$("$vswhere" -latest -products '*' -property installationPath | tr -d '\r')")"
crt="$(ls -d "$vs"/VC/Redist/MSVC/1*/x64/Microsoft.VC*.CRT 2>/dev/null | sort -V | tail -1)"
if [ -z "$crt" ]; then
    echo "no encuentro el runtime de Visual C++ redistribuible en $vs/VC/Redist" >&2
    exit 1
fi
rm -rf "$runtime"
mkdir -p "$runtime"
cp "$crt/vcruntime140.dll" "$crt/vcruntime140_1.dll" "$runtime/"

(cd "$root/rfirma-app" && pnpm exec tauri build --bundles nsis --config "$root/packaging/windows/tauri.windows.json")

installer="$(find "$CARGO_TARGET_DIR/release/bundle/nsis" -maxdepth 1 -type f -name '*-setup.exe' | sort | tail -1)"
if [ -z "$installer" ]; then
    echo "el bundler no produjo ningun instalador en $CARGO_TARGET_DIR/release/bundle/nsis" >&2
    exit 1
fi
echo "nsis: $installer ($(du -h "$installer" | cut -f1))"
