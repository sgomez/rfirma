#!/usr/bin/env bash
# Como build-native-fonts.sh pero anadiendo los metadatos de alcanzabilidad de
# AWT del agente de trazado (necesarios para la rubrica de IMAGEN en CE 25).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BRIDGE="$ROOT/rfirma-native-bridge"
DIR="$BRIDGE/target/${1:-ce25-awt}"
CFG="$BRIDGE/target/${2:-awt-config}"
GRAAL="${GRAALVM_HOME:-$HOME/.sdkman/candidates/java/25.3.4+1.r25-graalce}"

mkdir -p "$DIR"
cd "$DIR"
"$GRAAL/bin/native-image" --shared -H:Name=librfirma_crypto --no-fallback \
    -Djava.awt.headless=true \
    "-H:IncludeResources=com/lowagie/text/pdf/fonts/.*" \
    "-H:ConfigurationFileDirectories=$CFG" \
    -cp "$BRIDGE/target/rfirma-native-bridge-0.1.0.jar:$(cat "$BRIDGE/target/cp.txt")" \
    2>&1 | tee build.log | tail -8

ls -la "$DIR"/librfirma_crypto.so
