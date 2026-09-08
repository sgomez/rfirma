#!/usr/bin/env bash
# Compila el spike --shared, con el mismo -H:Name que el bloque de rfirma-
# native-bridge, para comparar tamanos con librfirma_crypto.so.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
GRAALVM_HOME="${GRAALVM_HOME:-$HOME/.sdkman/candidates/java/25.3.4+1.r25-graalce}"
OUT="${1:-$ROOT/target/native-shared}"
mkdir -p "$OUT"
cd "$OUT"

"$GRAALVM_HOME/bin/native-image" --shared -H:Name=libxades_spike --no-fallback \
  -H:ConfigurationFileDirectories="$ROOT/target/agent-config" \
  -cp "$ROOT/target/xades-spike-0.1.0.jar:$(cat "$ROOT/target/cp.txt")"

ls -la "$OUT"
