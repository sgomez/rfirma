#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
guard="$root/scripts/check-versions.sh"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

tree() {
    local dir="$work/$1"
    mkdir -p "$dir/.github/workflows" "$dir/scripts" "$dir/rfirma-native-bridge/testbench/reference-signer"
    printf '%s\n' AUTOFIRMA_VERSION=1.9.2 RUFF_VERSION=0.16.6 >"$dir/versions.env"
    printf 'maven := "mvn -Dautofirma.version=" + env("AUTOFIRMA_VERSION")\n' >"$dir/justfile"
    printf 'env:\n  X: ${{ env.RUFF_VERSION }}\n  Y: 10.16.6\n  Z: 0.16.60\n' >"$dir/.github/workflows/ci.yml"
    printf 'echo "autoscript-$AUTOFIRMA_VERSION.js"\n' >"$dir/scripts/a.sh"
    local pom
    for pom in rfirma-native-bridge/pom.xml rfirma-native-bridge/testbench/reference-signer/pom.xml; do
        printf '<properties>\n    <autofirma.version>1.9.2</autofirma.version>\n</properties>\n' >"$dir/$pom"
    done
    printf '%s\n' "$dir"
}

passes() {
    local case="$1" dir="$2"
    if ! "$guard" "$dir" >"$work/out" 2>&1; then
        echo "FALLO ($case): la guarda rechaza un arbol coherente" >&2
        cat "$work/out" >&2
        exit 1
    fi
}

fails_naming() {
    local case="$1" dir="$2" expected="$3"
    if "$guard" "$dir" >"$work/out" 2>&1; then
        echo "FALLO ($case): la guarda deja pasar el arbol" >&2
        exit 1
    fi
    if ! grep -qF -- "$expected" "$work/out"; then
        echo "FALLO ($case): esperaba '$expected' en" >&2
        cat "$work/out" >&2
        exit 1
    fi
}

passes "arbol coherente" "$(tree ok)"

dir="$(tree workflow)"
printf '        run: pipx install "ruff==0.16.6"\n' >>"$dir/.github/workflows/ci.yml"
fails_naming "literal en un workflow" "$dir" ".github/workflows/ci.yml:5:"

dir="$(tree etiqueta)"
printf 'url="https://example.org/v1.9.2/autoscript.js"\n' >>"$dir/scripts/a.sh"
fails_naming "etiqueta en un script" "$dir" "literal de AUTOFIRMA_VERSION"

dir="$(tree nombre)"
printf 'destino="autoscript-1.9.2.js"\n' >>"$dir/justfile"
fails_naming "version dentro de un nombre de fichero" "$dir" "justfile:2:"

dir="$(tree pom)"
sed -i 's/1\.9\.2/1.9.1/' "$dir/rfirma-native-bridge/testbench/reference-signer/pom.xml"
fails_naming "pom desincronizado" "$dir" "reference-signer/pom.xml: autofirma.version vale '1.9.1'"

dir="$(tree comillas)"
printf 'JUST_VERSION="1.45.0"\n' >>"$dir/versions.env"
fails_naming "valor con comillas" "$dir" "no es CLAVE=valor"

dir="$(tree export)"
printf 'export JUST_VERSION=1.45.0\n' >>"$dir/versions.env"
fails_naming "linea con export" "$dir" "no es CLAVE=valor"

dir="$(tree graalvm)"
printf '25.3.4.1\n' >"$dir/.graalvm-version"
fails_naming "fichero viejo de GraalVM" "$dir" ".graalvm-version"

echo "check-versions: correcto"
