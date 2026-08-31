#!/usr/bin/env bash
# Control en JVM normal: prefirma -> firma PK1 -> postfirma, sin native-image.
# Sirve para distinguir un fallo de native-image de un fallo de AutoFirma o del
# montaje de la prueba (mismo metodo que el issue #2).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BRIDGE="$ROOT/rfirma-native-bridge"
FIX="$ROOT/target/fixtures"
OUT="$BRIDGE/target/jvmctl"
JAVA="${GRAALVM_HOME:-$HOME/.sdkman/candidates/java/21-graalce}/bin/java"
CP="$BRIDGE/target/rfirma-native-bridge-0.1.0.jar:$(cat "$BRIDGE/target/cp.txt")"
EXTRA="${1:-}"

mkdir -p "$OUT"
cd "$OUT"

echo "== prefirma (JVM)"
"$JAVA" -Djava.awt.headless=true -cp "$CP" es.gob.afirma.nativebridge.NativeBridge \
    presign "$FIX/test.pdf.b64" "$FIX/cert.b64" $EXTRA > jvm-presign.xml

echo "== firma PK1 con la clave RSA de prueba"
python3 "$BRIDGE/testbench/inject-pk1.py" jvm-presign.xml "$FIX/key.pem" jvm-signed.xml

echo "== postfirma (JVM)"
"$JAVA" -Djava.awt.headless=true -cp "$CP" es.gob.afirma.nativebridge.NativeBridge \
    postsign "$FIX/test.pdf.b64" "$FIX/cert.b64" jvm-signed.xml $EXTRA
