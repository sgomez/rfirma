#!/usr/bin/env bash
# Construye la imagen nativa compartida con los mismos flags que el issue #2,
# en el directorio que se le pase (por defecto target/native-post).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BRIDGE="$ROOT/rfirma-native-bridge"
DIR="$BRIDGE/target/${1:-native-post}"
GRAAL="${GRAALVM_HOME:-$HOME/.sdkman/candidates/java/21-graalce}"

mkdir -p "$DIR"
cd "$DIR"
"$GRAAL/bin/native-image" --shared -H:Name=librfirma_crypto --no-fallback \
    -H:+PrintAnalysisCallTree \
    -cp "$BRIDGE/target/rfirma-native-bridge-0.1.0.jar:$(cat "$BRIDGE/target/cp.txt")" \
    2>&1 | tee build.log | tail -25

ls -la "$DIR"/librfirma_crypto.so
