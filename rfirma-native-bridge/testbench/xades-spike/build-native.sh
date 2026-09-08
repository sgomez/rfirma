#!/usr/bin/env bash
# Compila el spike a un ejecutable nativo (no --shared: no hay @CEntryPoint
# aqui, solo se mide si XAdES compila con native-image y cuanto pesa).
# Uso: build-native.sh <dir-salida>
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
GRAALVM_HOME=/home/sergio/.sdkman/candidates/java/25.3.4+1.r25-graalce
OUT="${1:-$ROOT/target/native}"
mkdir -p "$OUT"

"$GRAALVM_HOME/bin/native-image" \
  -cp "$ROOT/target/classes:$ROOT/target/xades-spike-0.1.0.jar:$(cat "$ROOT/target/cp.txt")" \
  -H:ConfigurationFileDirectories="$ROOT/target/agent-config" \
  --no-fallback \
  -o "$OUT/xades-spike" \
  es.gob.afirma.xadesspike.Main

ls -la "$OUT"
