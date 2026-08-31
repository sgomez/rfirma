#!/usr/bin/env bash
# Genera los metadatos de alcanzabilidad de AWT con el agente de trazado de
# GraalVM, ejercitando en JVM las DOS fases (prefirma y postfirma) con rubrica
# de imagen. Salida: target/awt-config/reachability-metadata.json
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BRIDGE="$ROOT/rfirma-native-bridge"
FIX="$ROOT/target/fixtures"
OUT="$BRIDGE/target/${2:-awt-config}"
WORK="$BRIDGE/target/trace"
EXTRA="${1:-$FIX/visible-imagen.properties}"
GRAAL="${GRAALVM_HOME:-$HOME/.sdkman/candidates/java/25.3.4+1.r25-graalce}"
CP="$BRIDGE/target/rfirma-native-bridge-0.1.0.jar:$(cat "$BRIDGE/target/cp.txt")"

rm -rf "$OUT" "$WORK"; mkdir -p "$OUT" "$WORK"
cd "$WORK"

env -u DISPLAY "$GRAAL/bin/java" -Djava.awt.headless=true \
    "-agentlib:native-image-agent=config-output-dir=$OUT" -cp "$CP" \
    es.gob.afirma.nativebridge.NativeBridge presign \
    "$FIX/test.pdf.b64" "$FIX/cert.b64" "$EXTRA" > presign.xml

# instantanea de los metadatos que produce SOLO la prefirma, para poder
# compararlos con los que anade ademas la postfirma.
cp -r "$OUT" "$OUT-pre"

python3 "$BRIDGE/testbench/inject-pk1.py" presign.xml "$FIX/key.pem" signed.xml

env -u DISPLAY "$GRAAL/bin/java" -Djava.awt.headless=true \
    "-agentlib:native-image-agent=config-merge-dir=$OUT" -cp "$CP" \
    es.gob.afirma.nativebridge.NativeBridge postsign \
    "$FIX/test.pdf.b64" "$FIX/cert.b64" signed.xml "$EXTRA"

ls -la "$OUT"
