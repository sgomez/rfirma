#!/usr/bin/env bash
# Oraculo de la grada C: valida <fichero> con SignValiderFactory del original
# 1.9.2 (afirma-crypto-validation), consumido desde Maven local (ADR-0002).
# Imprime VALID o INVALID <motivo> y sale con 0 o 1 respectivamente.
#
# Uso: validate.sh <fichero>
set -euo pipefail

if [ $# -ne 1 ]; then
    echo "Uso: validate.sh <fichero>" >&2
    exit 2
fi

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SIGNER="$ROOT/rfirma-native-bridge/testbench/reference-signer"

# La frescura se mide por contenido, no por mtime: un target restaurado de la cache del CI es mas
# viejo que el checkout y con -nt se recompilaria siempre.
STAMP="$SIGNER/target/.stamp"
WANTED="$(cat "$SIGNER/SignatureValidator.java" "$SIGNER/pom.xml" | sha256sum | cut -d' ' -f1)"

is_built() {
    [ -f "$SIGNER/target/classes/SignatureValidator.class" ] || return 1
    [ "$(cat "$STAMP" 2>/dev/null)" = "$WANTED" ] || return 1
    tr ':' '\n' < "$SIGNER/target/cp.txt" | while read -r jar; do
        [ -f "$jar" ] || exit 1
    done
}

if ! is_built; then
    mkdir -p "$SIGNER/target"
    (
        flock 200
        if ! is_built; then
            mvn -q -B -f "$SIGNER/pom.xml" dependency:build-classpath \
                -Dmdep.outputFile="$SIGNER/target/cp.txt" -Dmdep.includeScope=compile
            mkdir -p "$SIGNER/target/classes"
            javac -cp "$(cat "$SIGNER/target/cp.txt")" -d "$SIGNER/target/classes" \
                "$SIGNER/SignatureValidator.java"
            echo "$WANTED" > "$STAMP"
        fi
    ) 200>"$SIGNER/target/.build.lock"
fi

java -cp "$SIGNER/target/classes:$(cat "$SIGNER/target/cp.txt")" \
    SignatureValidator "$1"
