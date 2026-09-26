#!/usr/bin/env bash
# Regenera testdata/previous-signatures/ con el firmador monofasico PAdES del
# original 1.9.2 (AOPDFSigner), consumido desde Maven local (ADR-0002), y el
# sello de tiempo de una TSA de OpenSSL en el bucle local (ADR-0030).
#
# No es determinista: el PDF de entrada, el instante de firma y el sello
# cambian en cada regeneracion.
#
# Uso: make-previous-signature-samples.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BENCH="$ROOT/rfirma-native-bridge/testbench"
SIGNER="$BENCH/reference-signer"
OUT="$ROOT/testdata/previous-signatures"
EXPIRED="$ROOT/testdata/fnmt/expired-rsa.p12"
EXPIRED_PIN='G5cp,fYC9gje'

WORK="$(mktemp -d)"
TSA_PID=""
cleanup() {
    if [ -n "$TSA_PID" ]; then kill "$TSA_PID" 2>/dev/null || true; fi
    rm -rf "$WORK"
}
trap cleanup EXIT

mkdir -p "$OUT"

echo "== Classpath (Maven local, ADR-0002)"
mvn -q -B -f "$SIGNER/pom.xml" dependency:build-classpath \
    -Dmdep.outputFile="$SIGNER/target/cp.txt" -Dmdep.includeScope=compile
mkdir -p "$SIGNER/target/classes"
javac -cp "$(cat "$SIGNER/target/cp.txt")" -d "$SIGNER/target/classes" \
    "$SIGNER/ReferenceSigner.java"
RUN_CP="$SIGNER/target/classes:$(cat "$SIGNER/target/cp.txt")"

echo "== TSA de OpenSSL en el bucle local"
python3 "$BENCH/openssl-tsa.py" "$WORK/tsa" "$WORK/tsa.port" &
TSA_PID=$!
for _ in $(seq 50); do
    [ -s "$WORK/tsa.port" ] && break
    kill -0 "$TSA_PID"
    sleep 0.2
done
TSA_URL="http://127.0.0.1:$(cat "$WORK/tsa.port")/tsa"

sign() {
    echo "-- $*"
    java -cp "$RUN_CP" ReferenceSigner "$@"
}

sign pdf "$WORK/document.pdf"
sign pades-timestamped "$WORK/document.pdf" "$EXPIRED" "$EXPIRED_PIN" "$TSA_URL" \
    "$OUT/pades-long-term-expired.pdf"

ls -la "$OUT"
