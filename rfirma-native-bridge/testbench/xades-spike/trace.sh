#!/usr/bin/env bash
# Genera los metadatos de alcanzabilidad ejercitando en JVM la prefirma y la
# postfirma XAdES, con RSA y con EC, sobre la misma configuracion de salida
# (asi la traza cubre las dos ramas de KeyHelperFactory).
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
GRAALVM_HOME=/home/sergio/.sdkman/candidates/java/25.3.4+1.r25-graalce
CP="$ROOT/target/classes:$ROOT/target/xades-spike-0.1.0.jar:$(cat "$ROOT/target/cp.txt")"
CONFIG_DIR="$ROOT/target/agent-config"
rm -rf "$CONFIG_DIR"
mkdir -p "$CONFIG_DIR"

"$GRAALVM_HOME/bin/java" "-agentlib:native-image-agent=config-merge-dir=$CONFIG_DIR" \
  -cp "$CP" es.gob.afirma.xadesspike.Main \
  "$ROOT/../../../target/fixtures/rsa.p12" rsa 1234 SHA256withRSA \
  /tmp/xades-in.xml /tmp/xades-out-jvm.xml

"$GRAALVM_HOME/bin/java" "-agentlib:native-image-agent=config-merge-dir=$CONFIG_DIR" \
  -cp "$CP" es.gob.afirma.xadesspike.Main \
  "$ROOT/../../../target/fixtures/ec.p12" ec 1234 SHA256withECDSA \
  /tmp/xades-in.xml /tmp/xades-out-jvm-ec.xml

echo "Config en $CONFIG_DIR"
ls -la "$CONFIG_DIR"
