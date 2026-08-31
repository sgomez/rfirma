#!/usr/bin/env bash
# Equivalencia bit a bit: la MISMA sesion trifasica (mismo PRE, PK1, PID y TIME)
# ensamblada por la imagen nativa y por la JVM debe dar el mismo PDF.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BRIDGE="$ROOT/rfirma-native-bridge"
FIX="$ROOT/target/fixtures"
LAB="$BRIDGE/target/lab"
JAVA="${GRAALVM_HOME:-$HOME/.sdkman/candidates/java/21-graalce}/bin/java"
CP="$BRIDGE/target/rfirma-native-bridge-0.1.0.jar:$(cat "$BRIDGE/target/cp.txt")"

cd "$LAB"
env -i PATH=/usr/bin:/bin HOME=/tmp ./loader ./librfirma_crypto.so postsign \
    "$FIX/test.pdf.b64" "$FIX/cert.b64" signed.xml 2>/dev/null | tail -2
"$JAVA" -cp "$CP" es.gob.afirma.nativebridge.NativeBridge \
    postsign "$FIX/test.pdf.b64" "$FIX/cert.b64" signed.xml 2>/dev/null
sha256sum postsign.pdf jvm-postsign.pdf
cmp postsign.pdf jvm-postsign.pdf && echo "IDENTICOS" || echo "DIFIEREN"
