#!/usr/bin/env bash
# Igual que build-native.sh pero incluyendo los .afm de iText, para poder
# llegar mas alla del fallo "Courier not found as resource" y ver que hace la
# postfirma con rubrica visible en el punto siguiente (configuracion 2 de #2).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BRIDGE="$ROOT/rfirma-native-bridge"
DIR="$BRIDGE/target/${1:-native-post-fonts}"
GRAAL="${GRAALVM_HOME:-$HOME/.sdkman/candidates/java/21-graalce}"

mkdir -p "$DIR"
cd "$DIR"
"$GRAAL/bin/native-image" --shared -H:Name=librfirma_crypto --no-fallback \
    -Djava.awt.headless=true \
    "-H:IncludeResources=com/lowagie/text/pdf/fonts/.*" \
    -cp "$BRIDGE/target/rfirma-native-bridge-0.1.0.jar:$(cat "$BRIDGE/target/cp.txt")" \
    2>&1 | tee build.log | tail -8

ls -la "$DIR"/librfirma_crypto.so
