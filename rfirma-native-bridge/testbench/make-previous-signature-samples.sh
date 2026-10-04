#!/usr/bin/env bash
# Regenera testdata/previous-signatures/ con los firmadores PAdES y CAdES de AutoFirma y una TSA local (ADR-0002, ADR-0030).
#
# No es determinista: el PDF de entrada, el instante de firma y el sello
# cambian en cada regeneracion.
#
# Uso: make-previous-signature-samples.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BENCH="$ROOT/rfirma-native-bridge/testbench"
SIGNER="$BENCH/reference-signer"
AUTOFIRMA_VERSION="$("$ROOT/scripts/pinned-version.sh" AUTOFIRMA_VERSION)"
OUT="$ROOT/testdata/previous-signatures"
EXPIRED="$ROOT/testdata/fnmt/expired-rsa.p12"
EXPIRED_PIN='G5cp,fYC9gje'
ACTIVE="$ROOT/testdata/fnmt/active-rsa.p12"
ACTIVE_PIN='1234'
PSEUDONYM="$ROOT/testdata/fnmt/pseudonym-rsa.p12"
PSEUDONYM_PIN='1234'

WORK="$(mktemp -d)"
TSA_PID=""
cleanup() {
    if [ -n "$TSA_PID" ]; then kill "$TSA_PID" 2>/dev/null || true; fi
    rm -rf "$WORK"
}
trap cleanup EXIT

mkdir -p "$OUT"

echo "== Classpath (Maven local, ADR-0002)"
mvn -q -B -f "$SIGNER/pom.xml" -Dautofirma.version="$AUTOFIRMA_VERSION" dependency:build-classpath \
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
sign pades-timestamped "$WORK/document.pdf" "$ACTIVE" "$ACTIVE_PIN" "$TSA_URL" \
    "$OUT/pades-long-term-active.pdf"
sign pades-stamped-at "$WORK/document.pdf" "$EXPIRED" "$EXPIRED_PIN" "2019-06-01T00:00:00Z" \
    "$OUT/pades-stamped-while-in-force.pdf"

sign pades "$WORK/document.pdf" "$ACTIVE" "$ACTIVE_PIN" "$WORK/certified.pdf" \
    certificationLevel=1
sleep 1.1
sign pades "$WORK/certified.pdf" "$PSEUDONYM" "$PSEUDONYM_PIN" \
    "$OUT/pades-certified-then-cosigned.pdf" allowSigningCertifiedPdfs=true

sign pades "$WORK/document.pdf" "$ACTIVE" "$ACTIVE_PIN" "$WORK/forms-allowed.pdf" \
    certificationLevel=2
sleep 1.1
sign pades "$WORK/forms-allowed.pdf" "$PSEUDONYM" "$PSEUDONYM_PIN" "$WORK/closed.pdf" \
    certificationLevel=1 allowSigningCertifiedPdfs=true
sleep 1.1
sign pades "$WORK/closed.pdf" "$EXPIRED" "$EXPIRED_PIN" \
    "$OUT/pades-certified-twice-then-cosigned-expired.pdf" allowSigningCertifiedPdfs=true

sign pdf "$WORK/with-field.pdf" EarlierSignature
sign pades "$WORK/with-field.pdf" "$ACTIVE" "$ACTIVE_PIN" "$WORK/signed-in-field.pdf" \
    signatureField=EarlierSignature
sleep 1.1
sign pades "$WORK/signed-in-field.pdf" "$PSEUDONYM" "$PSEUDONYM_PIN" \
    "$OUT/pades-two-signatures-oldest-listed-first.pdf" signaturePage=1 \
    signaturePositionOnPageLowerLeftX=100 signaturePositionOnPageLowerLeftY=400 \
    signaturePositionOnPageUpperRightX=300 signaturePositionOnPageUpperRightY=500

printf 'rfirma: contenido firmado en CAdES\n' > "$WORK/content.txt"
sign cades implicit "$WORK/content.txt" "$EXPIRED" "$EXPIRED_PIN" "$OUT/cades-expired.csig"
sign cades implicit "$WORK/content.txt" "$ACTIVE" "$ACTIVE_PIN" "$WORK/cades-active.csig"
sign countersign cades tree "$WORK/cades-active.csig" "$EXPIRED" "$EXPIRED_PIN" \
    "$OUT/cades-countersigned-by-expired.csig"
echo "== CA autofirmada de pruebas, caducada desde 2015"
keytool -genkeypair -keystore "$WORK/expired-ca.p12" -storetype PKCS12 -storepass 123456 \
    -alias ca -keyalg RSA -keysize 2048 -dname "CN=rfirma CA caducada de pruebas, O=rfirma, C=ES" \
    -ext bc:c -startdate 2010/01/01 -validity 1826
keytool -exportcert -rfc -keystore "$WORK/expired-ca.p12" -storepass 123456 -alias ca \
    -file "$WORK/expired-ca.pem"
sign xades-extra-certificate "$ROOT/testdata/reference/document.xml" "$ACTIVE" "$ACTIVE_PIN" \
    "$WORK/expired-ca.pem" "$OUT/xades-expired-ca.xml"

ls -la "$OUT"
