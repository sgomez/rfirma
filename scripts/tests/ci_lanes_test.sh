#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
lanes="$root/scripts/ci-lanes.sh"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

printf '%s\n' \
    rfirma-app/src-tauri/src/desktop/adapters/paths.rs \
    rfirma-app/src-tauri/src/identity/adapters/windows_store/ \
    >"$work/platform-files"

check() {
    local case="$1" expected="$2" actual="$3"
    if [ "$actual" != "$expected " ]; then
        echo "FALLO ($case):" >&2
        echo "  esperado: $expected" >&2
        echo "  obtenido: $actual" >&2
        exit 1
    fi
}

expect() {
    local case="$1" expected="$2"
    shift 2
    check "$case" "$expected" "$(printf '%s\n' "$@" | "$lanes" | tr '\n' ' ')"
}

expect_with() {
    local case="$1" expected="$2" options="$3"
    shift 3
    # shellcheck disable=SC2086
    check "$case" "$expected" "$(printf '%s\n' "$@" | "$lanes" $options | tr '\n' ' ')"
}

linux="java=true web=true rust=true native=true landing=true"
all="$linux windows=true macos=true"
none="java=false web=false rust=false native=false landing=false windows=false macos=false"
backend="java=false web=false rust=true native=true landing=false"

check "sin ficheros" "$all" "$("$lanes" </dev/null | tr '\n' ' ')"

expect "fichero desconocido" "$linux windows=false macos=false" "lefthook.yml"
expect "justfile" "$all" "justfile"
expect "workflow" "$all" ".github/workflows/ci.yml"
expect "accion local" "$all" ".github/actions/setup-runner/action.yml"
expect "prosa inerte" "$none" "README.md" "docs/research/x.md" "rfirma-conformance/src/main.rs"
expect "interfaz" "java=false web=true rust=true native=false landing=false windows=false macos=false" "rfirma-app/src/App.tsx"
expect "backend" "$backend windows=false macos=false" "rfirma-app/src-tauri/src/lib.rs"
expect "capacidad de tauri" "java=false web=true rust=true native=true landing=false windows=false macos=false" "rfirma-app/src-tauri/capabilities/site.json"
expect "candado de version" "java=false web=true rust=true native=true landing=false windows=true macos=true" "rfirma-app/src-tauri/Cargo.toml"
expect "candado de dependencias" "java=false web=true rust=true native=true landing=false windows=true macos=true" "rfirma-app/src-tauri/Cargo.lock"
expect "script de compilacion" "$backend windows=true macos=true" "rfirma-app/src-tauri/build.rs"
expect "configuracion de tauri" "java=false web=true rust=true native=true landing=false windows=true macos=true" "rfirma-app/src-tauri/tauri.conf.json"
expect "reglas de clippy" "$backend windows=true macos=true" "rfirma-app/src-tauri/clippy.toml"
expect "manifiesto de windows" "java=false web=true rust=true native=true landing=false windows=true macos=false" "rfirma-app/src-tauri/windows-app-manifest.xml"
expect "empaquetado de windows" "java=false web=true rust=false native=false landing=false windows=true macos=false" "packaging/windows/tauri.windows.json"
expect "empaquetado de macos" "java=false web=true rust=false native=false landing=false windows=false macos=true" "packaging/macos/Info.plist"
expect "puente" "java=true web=false rust=false native=true landing=false windows=false macos=false" "rfirma-native-bridge/src/main/java/A.java"
expect "banco de referencia" "java=true web=false rust=true native=true landing=false windows=false macos=false" "rfirma-native-bridge/testbench/validate.sh"
expect "pom" "java=true web=true rust=false native=true landing=false windows=false macos=false" "rfirma-native-bridge/pom.xml"
expect "adr" "java=false web=false rust=false native=false landing=false windows=false macos=false" "docs/adr/0001-x.md"
expect "diseno" "java=false web=true rust=false native=false landing=false windows=false macos=false" "docs/design/design-system.md"
expect "empaquetado" "java=false web=true rust=false native=false landing=false windows=false macos=false" "packaging/gnome/rfirma-sign.py"
expect "bootstrap" "java=true web=true rust=false native=true landing=false windows=true macos=true" "scripts/bootstrap.sh"
expect "lector de versiones" "java=true web=true rust=true native=true landing=false windows=true macos=true" "scripts/pinned-version.sh"
expect "instalador de herramientas" "java=false web=true rust=true native=true landing=false windows=true macos=true" "scripts/install-tools.sh"
expect "versiones fijadas" "java=true web=true rust=true native=true landing=true windows=true macos=true" "versions.env"
expect "las propias reglas" "$all" "scripts/ci-lanes.sh"
expect "la lista de plataforma" "java=false web=true rust=false native=true landing=false windows=true macos=true" "scripts/platform-files.sh"
expect "otro script" "java=false web=true rust=false native=true landing=false windows=false macos=false" "scripts/token-per-test.sh"
expect "site-driver" "java=false web=true rust=true native=true landing=false windows=false macos=false" "testdata/site-driver/driver.mjs"
expect "kit fnmt" "java=true web=false rust=true native=true landing=false windows=false macos=false" "testdata/fnmt/README.md"
expect "landing" "java=false web=false rust=false native=false landing=true windows=false macos=false" "packaging/repo/site/src/pages/index.astro"
expect "imagen de la landing" "java=false web=false rust=false native=false landing=true windows=false macos=false" "packaging/repo/Dockerfile"
expect "sistema de diseno" "java=false web=true rust=true native=false landing=true windows=false macos=false" "rfirma-app/src/design-system/bundle/styles.css"
expect "consola" "java=false web=true rust=false native=false landing=false windows=false macos=false" "rfirma-conformance/console/src/App.tsx"
expect "raiz del workspace" "$linux windows=false macos=false" "pnpm-lock.yaml"
expect "la union de dos" "java=false web=true rust=true native=true landing=false windows=false macos=false" "rfirma-app/src/App.tsx" "rfirma-app/src-tauri/src/lib.rs"

platforms="--platform-files $work/platform-files"
expect_with "fichero con cfg de plataforma" "$backend windows=true macos=true" "$platforms" \
    "rfirma-app/src-tauri/src/desktop/adapters/paths.rs"
expect_with "modulo que solo compila una plataforma" "$backend windows=true macos=true" "$platforms" \
    "rfirma-app/src-tauri/src/identity/adapters/windows_store/cng.rs"
expect_with "prefijo sin su barra" "$backend windows=false macos=false" "$platforms" \
    "rfirma-app/src-tauri/src/identity/adapters/windows_store_other.rs"
expect_with "backend sin cfg" "$backend windows=false macos=false" "$platforms" \
    "rfirma-app/src-tauri/src/lib.rs"
expect_with "puente con la lista" "java=true web=false rust=false native=true landing=false windows=false macos=false" "$platforms" \
    "rfirma-native-bridge/src/main/java/A.java"

expect_with "push sin ficheros" "$linux windows=false macos=false" "--no-platforms"
expect_with "push que cambia Cargo.lock" "java=false web=true rust=true native=true landing=false windows=false macos=false" "--no-platforms" \
    "rfirma-app/src-tauri/Cargo.lock"
expect_with "etiqueta de windows" "java=false web=true rust=true native=false landing=false windows=true macos=false" "--force windows" \
    "rfirma-app/src/App.tsx"
expect_with "etiquetas de las dos" "java=false web=false rust=false native=false landing=false windows=true macos=true" "--force windows --force macos" \
    "README.md"

if "$lanes" --force linux </dev/null >/dev/null 2>&1; then
    echo "FALLO (carril forzado desconocido): se aceptó" >&2
    exit 1
fi

echo "ci_lanes_test: correcto"
