#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
lanes="$root/scripts/ci-lanes.sh"

expect() {
    local case="$1" expected="$2"
    shift 2
    local actual
    actual="$(printf '%s\n' "$@" | "$lanes" | tr '\n' ' ')"
    if [ "$actual" != "$expected " ]; then
        echo "FALLO ($case):" >&2
        echo "  esperado: $expected" >&2
        echo "  obtenido: $actual" >&2
        exit 1
    fi
}

all="java=true web=true rust=true native=true"
none="java=false web=false rust=false native=false"

actual="$("$lanes" </dev/null | tr '\n' ' ')"
[ "$actual" = "$all " ] || { echo "FALLO (sin ficheros): $actual" >&2; exit 1; }

expect "fichero desconocido" "$all" "lefthook.yml"
expect "justfile" "$all" "justfile"
expect "workflow" "$all" ".github/workflows/ci.yml"
expect "prosa inerte" "$none" "README.md" "docs/research/x.md" "rfirma-conformance/src/main.rs" "changelog.d/1.md"
expect "interfaz" "java=false web=true rust=true native=false" "rfirma-app/src/App.tsx"
expect "backend" "java=false web=false rust=true native=true" "rfirma-app/src-tauri/src/lib.rs"
expect "capacidad de tauri" "java=false web=true rust=true native=true" "rfirma-app/src-tauri/capabilities/site.json"
expect "candado de version" "java=false web=true rust=true native=true" "rfirma-app/src-tauri/Cargo.toml"
expect "puente" "java=true web=false rust=false native=true" "rfirma-native-bridge/src/main/java/A.java"
expect "banco de referencia" "java=true web=false rust=true native=true" "rfirma-native-bridge/testbench/validate.sh"
expect "pom" "java=true web=true rust=false native=true" "rfirma-native-bridge/pom.xml"
expect "adr" "java=false web=false rust=true native=false" "docs/adr/0001-x.md"
expect "diseno" "java=false web=true rust=false native=false" "docs/design/design-system.md"
expect "empaquetado" "java=false web=true rust=false native=false" "packaging/gnome/rfirma-sign.py"
expect "bootstrap" "java=true web=true rust=false native=true" "scripts/bootstrap.sh"
expect "las propias reglas" "$all" "scripts/ci-lanes.sh"
expect "otro script" "java=false web=true rust=false native=true" "scripts/token-per-test.sh"
expect "site-driver" "java=false web=true rust=true native=true" "testdata/site-driver/driver.mjs"
expect "kit fnmt" "java=true web=false rust=true native=true" "testdata/fnmt/README.md"
expect "la union de dos" "java=false web=true rust=true native=true" "rfirma-app/src/App.tsx" "rfirma-app/src-tauri/src/lib.rs"

echo "ci_lanes_test: correcto"
