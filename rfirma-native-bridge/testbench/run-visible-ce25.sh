#!/usr/bin/env bash
# Flujo trifasico COMPLETO (prefirma + postfirma) con rubrica visible en la
# imagen nativa de GraalVM CE 25, y control bit a bit contra la JVM.
#
# Uso: run-visible-ce25.sh <dir-imagen> <fichero.properties> <etiqueta> [n-so]
#   n-so = "1"  -> solo librfirma_crypto.so (rubrica de texto)
#   n-so = "6"  -> mas libawt/libawt_headless/libjavajpeg/libjava/libjvm
#
# Los extraParams y el TriphaseData son EXACTAMENTE los mismos en las dos
# fases y en los dos motores: la restriccion dura de #13 (extraParams o TIME
# distintos invalidan la firma en silencio) obliga a ello.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BRIDGE="$ROOT/rfirma-native-bridge"
FIX="$ROOT/target/fixtures"
SRC="$BRIDGE/target/$1"
EXTRA="$2"
TAG="$3"
NSO="${4:-1}"
LAB="$BRIDGE/target/lab-$TAG"
GRAAL="${GRAALVM_HOME:-$HOME/.sdkman/candidates/java/25.3.4+1.r25-graalce}"
CP="$BRIDGE/target/rfirma-native-bridge-0.1.0.jar:$(cat "$BRIDGE/target/cp.txt")"

rm -rf "$LAB"; mkdir -p "$LAB"
cp "$SRC/librfirma_crypto.so" "$LAB/"
if [ "$NSO" = 6 ]; then
    for l in libawt.so libawt_headless.so libjavajpeg.so libjava.so libjvm.so; do
        cp "$SRC/$l" "$LAB/"
    done
fi
gcc -O2 -o "$LAB/loader" "$BRIDGE/testbench/loader.c" -ldl
cd "$LAB"

echo "== .so en el directorio: $(ls ./*.so | wc -l)"
echo "== prefirma (NATIVO, env -i)"
env -i PATH=/usr/bin:/bin HOME=/tmp \
    ./loader ./librfirma_crypto.so presign "$FIX/test.pdf.b64" "$FIX/cert.b64" "$EXTRA" \
    | tail -2

echo "== firma PK1"
python3 "$BRIDGE/testbench/inject-pk1.py" presign.xml "$FIX/key.pem" signed.xml

echo "== postfirma (NATIVO, env -i, mismos extraParams y mismo TIME)"
env -i PATH=/usr/bin:/bin HOME=/tmp \
    ./loader ./librfirma_crypto.so postsign "$FIX/test.pdf.b64" "$FIX/cert.b64" \
    signed.xml "$EXTRA" | tail -2
mv postsign.pdf "$TAG-nativo.pdf"

echo "== control JVM con la MISMA sesion trifasica"
env -u DISPLAY "$GRAAL/bin/java" -Djava.awt.headless=true -cp "$CP" \
    es.gob.afirma.nativebridge.NativeBridge postsign "$FIX/test.pdf.b64" \
    "$FIX/cert.b64" signed.xml "$EXTRA" 2>/dev/null

echo "== equivalencia bit a bit"
sha256sum "$TAG-nativo.pdf" jvm-postsign.pdf
cmp "$TAG-nativo.pdf" jvm-postsign.pdf && echo IDENTICOS || echo DIFIEREN

echo "== validacion"
pdfsig "$TAG-nativo.pdf" | grep -E "Signature Validation|Total document|Signed Ranges"
