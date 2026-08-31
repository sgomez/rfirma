#!/usr/bin/env bash
# PROTOTIPO #9 — firma un caso con los extraParams que produjo el visor.
#
#   ./firmar.sh salidas/a4-rot90.properties
#
# Lee el caso del comentario "# rfirma-esperado:" del propio .properties, hace
# el ciclo trifasico entero (prefirma -> PK1 -> postfirma) sobre la JVM de
# GraalVM CE 25 y deja el PDF firmado junto al .properties.
set -euo pipefail
D="$(cd "$(dirname "$0")" && pwd)"
PROPS="$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"
CASO=$(grep -o '"caso":"[^"]*"' "$PROPS" | cut -d'"' -f4)
PDF="$D/casos/$CASO.pdf"
GRAAL="${GRAALVM_HOME:-$HOME/.sdkman/candidates/java/25.3.4+1.r25-graalce}"
CP="$D/motor/rfirma-native-bridge-0.1.0.jar:$(cat "$D/motor/cp.txt")"
NOM="$(basename "$PROPS" .properties)"
LAB="$D/salidas/lab-$NOM"

rm -rf "$LAB"; mkdir -p "$LAB"; cd "$LAB"
base64 -w0 "$PDF" > pdf.b64

env -u DISPLAY "$GRAAL/bin/java" -Djava.awt.headless=true -cp "$CP" \
    es.gob.afirma.nativebridge.NativeBridge presign pdf.b64 "$D/motor/cert.b64" \
    "$PROPS" > presign.xml
grep -q '^ERROR:' presign.xml && { cat presign.xml; exit 1; }

python3 "$D/motor/inject-pk1.py" presign.xml "$D/motor/key.pem" firmado.xml >/dev/null

env -u DISPLAY "$GRAAL/bin/java" -Djava.awt.headless=true -cp "$CP" \
    es.gob.afirma.nativebridge.NativeBridge postsign pdf.b64 "$D/motor/cert.b64" \
    firmado.xml "$PROPS"

mv jvm-postsign.pdf "$D/salidas/$NOM-firmado.pdf"
echo "-> salidas/$NOM-firmado.pdf"
