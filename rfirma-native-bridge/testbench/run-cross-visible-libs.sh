#!/usr/bin/env bash
# Como run-cross-visible.sh pero con los 10 .so presentes y java.library.path
# apuntado a ellos (configuracion 4 del issue #2), para ver si la postfirma con
# rubrica visible llega al mismo aborto dentro del JNI_OnLoad de libawt.so.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BRIDGE="$ROOT/rfirma-native-bridge"
FIX="$ROOT/target/fixtures"
SRC="$BRIDGE/target/${1:-native-post-fonts}"
LAB2="$BRIDGE/target/lab2"

rm -rf "$LAB2"
mkdir -p "$LAB2"
cp "$SRC"/*.so "$LAB2/"
cp "$BRIDGE/target/lab/loader" "$LAB2/"
cp "$BRIDGE/target/lab/vis-signed.xml" "$LAB2/"
cd "$LAB2"

echo "== .so presentes: $(ls *.so | wc -l)"
echo "== postfirma CON rubrica visible, RFIRMA_LIB_DIR apuntado a este directorio"
set +e
env -i PATH=/usr/bin:/bin HOME=/tmp RFIRMA_LIB_DIR="$LAB2" LD_DEBUG=libs \
    ./loader ./librfirma_crypto.so postsign \
    "$FIX/test.pdf.b64" "$FIX/cert.b64" vis-signed.xml \
    "$FIX/visible-texto.properties" > vis.out 2> vis.err
echo "rc=$?"
set -e
echo "-- stdout"; cat vis.out
echo "-- ultimas lineas de LD_DEBUG"; tail -12 vis.err
echo "-- libs inicializadas"; grep -oP 'calling init: \K.*' vis.err | sort -u
