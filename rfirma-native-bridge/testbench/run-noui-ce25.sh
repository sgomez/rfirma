#!/usr/bin/env bash
# Ciclo trifasico completo con rubrica visible sobre una imagen nativa
# construida SIN afirma-ui-utils y SIN metadatos de AWT (issue #36).
#
# Uso: run-noui-ce25.sh <dir-imagen> <fichero.properties> <etiqueta> [lista-so]
#   lista-so: nombres de .so auxiliares a copiar junto a librfirma_crypto.so,
#             separados por espacios. Vacio (por defecto) = un solo fichero.
#
# Ademas del nativo ejecuta DOS controles en JVM: uno con afirma-ui-utils en
# el classpath (AutoFirma tal cual) y otro sin el (mismo recorte que la
# imagen). Asi se distingue "el nativo ensambla otro PDF" de "excluir el
# modulo ensambla otro PDF".
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BRIDGE="$ROOT/rfirma-native-bridge"
FIX="$ROOT/target/fixtures"
SRC="$BRIDGE/target/$1"
EXTRA="$2"
TAG="$3"
LIBS="${4:-}"
LAB="$BRIDGE/target/lab-$TAG"
GRAAL="${GRAALVM_HOME:-$HOME/.sdkman/candidates/java/25.3.4+1.r25-graalce}"
UIUTILS="$HOME/.m2/repository/es/gob/afirma/afirma-ui-utils/1.9.1/afirma-ui-utils-1.9.1.jar"
CP="$BRIDGE/target/rfirma-native-bridge-0.1.0.jar:$(cat "$BRIDGE/target/cp.txt")"

rm -rf "$LAB"; mkdir -p "$LAB"
cp "$SRC/librfirma_crypto.so" "$LAB/"
for l in $LIBS; do cp "$SRC/$l" "$LAB/"; done
gcc -O2 -o "$LAB/loader" "$BRIDGE/testbench/loader.c" -ldl
cd "$LAB"

echo "== .so en el directorio: $(ls ./*.so | wc -l) ($(ls ./*.so | tr '\n' ' '))"

echo "== prefirma (NATIVO, env -i)"
# El fallo de la prefirma es un resultado que hay que ver, no un fallo del
# banco: no debe tumbar el guion (rc=3 con error recuperable, 134 si aborta).
env -i PATH=/usr/bin:/bin HOME=/tmp \
    ./loader ./librfirma_crypto.so presign "$FIX/test.pdf.b64" "$FIX/cert.b64" "$EXTRA" \
    > presign.log 2>&1 || echo "prefirma rc=$?"
tail -3 presign.log

if ! [ -s presign.xml ] || grep -q "^ERROR:" presign.xml; then
    echo "== la prefirma no ha producido sesion: se para aqui (es un resultado, no un fallo del banco)"
    exit 0
fi

echo "== firma PK1"
python3 "$BRIDGE/testbench/inject-pk1.py" presign.xml "$FIX/key.pem" signed.xml

echo "== postfirma (NATIVO, env -i, mismos extraParams y mismo TIME)"
env -i PATH=/usr/bin:/bin HOME=/tmp \
    ./loader ./librfirma_crypto.so postsign "$FIX/test.pdf.b64" "$FIX/cert.b64" \
    signed.xml "$EXTRA" 2>&1 | tail -3
[ -f postsign.pdf ] && mv postsign.pdf "$TAG-nativo.pdf"

echo "== control JVM SIN afirma-ui-utils (mismo recorte que la imagen)"
env -u DISPLAY "$GRAAL/bin/java" -Djava.awt.headless=true -cp "$CP" \
    es.gob.afirma.nativebridge.NativeBridge postsign "$FIX/test.pdf.b64" \
    "$FIX/cert.b64" signed.xml "$EXTRA" 2>/dev/null
mv jvm-postsign.pdf jvm-sin-uiutils.pdf

echo "== control JVM CON afirma-ui-utils (AutoFirma tal cual)"
env -u DISPLAY "$GRAAL/bin/java" -Djava.awt.headless=true -cp "$CP:$UIUTILS" \
    es.gob.afirma.nativebridge.NativeBridge postsign "$FIX/test.pdf.b64" \
    "$FIX/cert.b64" signed.xml "$EXTRA" 2>/dev/null
mv jvm-postsign.pdf jvm-con-uiutils.pdf

if [ -f "$TAG-nativo.pdf" ]; then
    echo "== equivalencia bit a bit"
    sha256sum "$TAG-nativo.pdf" jvm-sin-uiutils.pdf jvm-con-uiutils.pdf
    cmp -s "$TAG-nativo.pdf" jvm-sin-uiutils.pdf \
        && echo "nativo == jvm-sin-uiutils: IDENTICOS" || echo "nativo == jvm-sin-uiutils: DIFIEREN"
    cmp -s "$TAG-nativo.pdf" jvm-con-uiutils.pdf \
        && echo "nativo == jvm-con-uiutils: IDENTICOS" || echo "nativo == jvm-con-uiutils: DIFIEREN"

    echo "== validacion"
    pdfsig "$TAG-nativo.pdf" | grep -E "Signature Validation|Total document"

    echo "== la rubrica se ve (rasterizado de la pagina 1)"
    # pdftoppm numera con tantos digitos como paginas tenga el PDF (22 -> -01).
    pdftoppm -f 1 -l 1 -r 100 -png "$TAG-nativo.pdf" "$TAG-firmado"
    pdftoppm -f 1 -l 1 -r 100 -png "$FIX/test.pdf" "$TAG-base"
    python3 "$BRIDGE/testbench/diff-pagina.py" \
        "$(ls "$TAG"-base-*.png)" "$(ls "$TAG"-firmado-*.png)" "$TAG-rubrica.png"
fi
