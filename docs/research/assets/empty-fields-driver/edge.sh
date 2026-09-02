#!/usr/bin/env bash
# Modos de fallo de `signatureField`: campo ya firmado y campo inexistente.
set -uo pipefail
cd "$(dirname "$0")"
J=/home/sergio/.sdkman/candidates/java/25.3.4+1.r25-graalce/bin/java
P12="$HOME/.local/share/rfirma-test-certs/Claves RSA/AC Sector Público/Empleado Público/SP_Empleado_publico_activo.p12"
CP="target/probe-1.jar:$(cat target/cp.txt)"

echo "== firmar sobre un campo YA firmado =="
"$J" -cp "$CP" probe.Probe sign signed-field.pdf "$P12" Firma2 /tmp/149-yafirmado.pdf 2>&1 | grep -v '^WARNING' | tail -6
echo "== firmar sobre un campo INEXISTENTE =="
"$J" -cp "$CP" probe.Probe sign empty-fields.pdf "$P12" NoExiste /tmp/149-inexistente.pdf 2>&1 | grep -v '^WARNING' | tail -6
