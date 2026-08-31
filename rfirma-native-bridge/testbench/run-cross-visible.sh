#!/usr/bin/env bash
# Prueba cruzada para aislar la postfirma CON rubrica visible.
#
# La prefirma con rubrica aborta en nativo (issue #2), asi que el PRE se genera
# en JVM, se firma con la clave de prueba, y solo la POSTFIRMA se ejecuta en la
# imagen nativa con los MISMOS extraParams. Asi se mide si la postfirma toca
# AWT por si misma.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BRIDGE="$ROOT/rfirma-native-bridge"
FIX="$ROOT/target/fixtures"
LAB="$BRIDGE/target/lab"
JAVA="${GRAALVM_HOME:-$HOME/.sdkman/candidates/java/21-graalce}/bin/java"
CP="$BRIDGE/target/rfirma-native-bridge-0.1.0.jar:$(cat "$BRIDGE/target/cp.txt")"
EXTRA="$FIX/visible-texto.properties"

cd "$LAB"

echo "== prefirma CON rubrica visible (JVM, headless)"
env -u DISPLAY "$JAVA" -Djava.awt.headless=true -cp "$CP" \
    es.gob.afirma.nativebridge.NativeBridge presign \
    "$FIX/test.pdf.b64" "$FIX/cert.b64" "$EXTRA" > vis-presign.xml

echo "== firma PK1"
python3 "$BRIDGE/testbench/inject-pk1.py" vis-presign.xml "$FIX/key.pem" vis-signed.xml

echo "== postfirma CON rubrica visible (NATIVO, env -i, un solo .so)"
set +e
env -i PATH=/usr/bin:/bin HOME=/tmp ./loader ./librfirma_crypto.so postsign \
    "$FIX/test.pdf.b64" "$FIX/cert.b64" vis-signed.xml "$EXTRA"
echo "rc-nativo=$?"
set -e

echo "== control: la misma postfirma en JVM"
env -u DISPLAY "$JAVA" -Djava.awt.headless=true -cp "$CP" \
    es.gob.afirma.nativebridge.NativeBridge postsign \
    "$FIX/test.pdf.b64" "$FIX/cert.b64" vis-signed.xml "$EXTRA"
