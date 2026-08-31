#!/usr/bin/env bash
# Prueba aislada de la imagen nativa: prefirma -> firma PK1 -> postfirma,
# con UN SOLO .so en el directorio y bajo `env -i` (sin JAVA_HOME, sin GraalVM
# ni JDK en el PATH). Mismo metodo que el issue #2.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BRIDGE="$ROOT/rfirma-native-bridge"
FIX="$ROOT/target/fixtures"
SRC="$BRIDGE/target/${1:-native-post}"
LAB="$BRIDGE/target/lab"
EXTRA="${2:-}"

rm -rf "$LAB"
mkdir -p "$LAB"
cp "$SRC/librfirma_crypto.so" "$LAB/"
gcc -O2 -o "$LAB/loader" "$BRIDGE/testbench/loader.c" -ldl
cd "$LAB"

echo "== contenido del directorio (debe ser solo el .so y el loader)"
ls -la

echo "== prefirma (nativo, env -i)"
env -i PATH=/usr/bin:/bin HOME=/tmp ./loader ./librfirma_crypto.so presign \
    "$FIX/test.pdf.b64" "$FIX/cert.b64" $EXTRA

echo "== firma PK1 con la clave RSA de prueba"
python3 "$BRIDGE/testbench/inject-pk1.py" presign.xml "$FIX/key.pem" signed.xml

echo "== postfirma (nativo, env -i)"
env -i PATH=/usr/bin:/bin HOME=/tmp ./loader ./librfirma_crypto.so postsign \
    "$FIX/test.pdf.b64" "$FIX/cert.b64" signed.xml $EXTRA
