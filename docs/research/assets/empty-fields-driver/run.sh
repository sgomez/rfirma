#!/usr/bin/env bash
# Sondeo #149. Reconstruye el banco entero: PDF con campos vacios, enumeracion
# con PdfUtil y dos firmas (con y sin `signatureField`), ambas enviando los
# mismos parametros de posicion "senuelo".
set -euo pipefail
cd "$(dirname "$0")"

J=/home/sergio/.sdkman/candidates/java/25.3.4+1.r25-graalce/bin/java
P12="$HOME/.local/share/rfirma-test-certs/Claves RSA/AC Sector Público/Empleado Público/SP_Empleado_publico_activo.p12"
CP="target/probe-1.jar:$(cat target/cp.txt)"

python3 mkpdf.py empty-fields.pdf
"$J" -cp "$CP" probe.Probe list empty-fields.pdf 2>&1 | grep -v '^WARNING'
"$J" -cp "$CP" probe.Probe sign empty-fields.pdf "$P12" Firma2 signed-field.pdf 2>&1 | grep -v '^WARNING' | tail -1
"$J" -cp "$CP" probe.Probe sign empty-fields.pdf "$P12" - signed-nofield.pdf 2>&1 | grep -v '^WARNING' | tail -1
"$J" -cp "$CP" probe.Probe sign empty-fields.pdf "$P12" FirmaInvisible signed-invisible.pdf 2>&1 | grep -v '^WARNING' | tail -1
"$J" -cp "$CP" probe.Probe list signed-field.pdf 2>&1 | grep -v '^WARNING'
node probe-pdfjs.mjs empty-fields.pdf
echo "--- vacio vs firmado, lo que ve pdf.js ---"
node compare-pdfjs.mjs
echo "--- lo que sí separa firmado de sin firmar: /ByteRange en los bytes crudos ---"
for f in empty-fields.pdf signed-field.pdf signed-nofield.pdf signed-invisible.pdf; do
    printf "%-22s /ByteRange=%s\n" "$f" "$(grep -ac '/ByteRange' "$f")"
done
