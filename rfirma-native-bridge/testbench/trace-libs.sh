#!/usr/bin/env bash
# Traza con LD_DEBUG=libs que librerias nativas carga realmente la postfirma
# (mismo metodo con el que el issue #2 situo el aborto dentro de libawt.so).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
FIX="$ROOT/target/fixtures"
LAB="$ROOT/rfirma-native-bridge/target/lab"
MODE="${1:-postsign}"

cd "$LAB"
if [ "$MODE" = postsign ]; then
    ARGS=("$FIX/test.pdf.b64" "$FIX/cert.b64" signed.xml)
else
    ARGS=("$FIX/test.pdf.b64" "$FIX/cert.b64")
fi

set +e
env -i PATH=/usr/bin:/bin HOME=/tmp LD_DEBUG=libs \
    ./loader ./librfirma_crypto.so "$MODE" "${ARGS[@]}" > ld-$MODE.out 2> ld-$MODE.err
echo "rc=$?"
set -e

echo "== librerias inicializadas durante $MODE"
grep -oP 'calling init: \K.*' "ld-$MODE.err" | sort -u
echo "== hay awt/fontmanager/lcms/javajpeg?"
grep -ciE 'libawt|libfontmanager|liblcms|libjavajpeg|libmlib' "ld-$MODE.err" || true
echo "== resultado"
tail -3 "ld-$MODE.out"
