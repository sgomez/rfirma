#!/usr/bin/env bash
# Comprueba la restriccion dura de docs/research/firma-visible-trifasica.md:
# si la postfirma no recibe el mismo instante de firma que la prefirma, el PDF
# sale igualmente pero la firma queda invalida SIN dar error.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
FIX="$ROOT/target/fixtures"
LAB="$ROOT/rfirma-native-bridge/target/lab"

cd "$LAB"
TIME=$(grep -oP '(?<=<param n="TIME">)[0-9]+' signed.xml)
sed "s|<param n=\"TIME\">$TIME</param>|<param n=\"TIME\">$((TIME + 60000))</param>|" \
    signed.xml > mismatch.xml
echo "TIME de la prefirma: $TIME -> postfirma con $((TIME + 60000))"

env -i PATH=/usr/bin:/bin HOME=/tmp ./loader ./librfirma_crypto.so postsign \
    "$FIX/test.pdf.b64" "$FIX/cert.b64" mismatch.xml 2>/dev/null | tail -2
mv postsign.pdf mismatch.pdf
echo "== validacion del PDF con TIME desparejado"
pdfsig mismatch.pdf | grep -E "Signature Validation|Total document|Signed Ranges"
