#!/usr/bin/env bash
# El crecimiento REAL: el .so de produccion (CAdES+PAdES, sin XAdES) mas el
# arbol de XAdES en el MISMO classpath y con los DOS @CEntryPoint presentes,
# para que native-image analice ambos arboles como lo haria un pom.xml real.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BRIDGE="$ROOT/../.."
GRAALVM_HOME=/home/sergio/.sdkman/candidates/java/25.3.4+1.r25-graalce
OUT="${1:-$ROOT/target/native-combined}"
mkdir -p "$OUT"
cd "$OUT"

"$GRAALVM_HOME/bin/native-image" --shared -H:Name=librfirma_crypto_xades --no-fallback \
  -H:ConfigurationFileDirectories="$ROOT/target/agent-config" \
  -cp "$BRIDGE/target/rfirma-native-bridge-0.1.0.jar:$(cat "$BRIDGE/target/cp.txt"):$ROOT/target/xades-spike-0.1.0.jar:$(cat "$ROOT/target/cp.txt")"

ls -la "$OUT"
