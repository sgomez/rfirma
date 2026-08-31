#!/usr/bin/env bash
# PROTOTIPO #9 — deja el motor de firma listo. Nada de esto se versiona: el
# .jar y cp.txt son de esta maquina, y la clave es una clave privada.
#
# Produce en este directorio: rfirma-native-bridge-0.1.0.jar, cp.txt,
# cert.b64 (certificado autofirmado de usar y tirar) y key.pem.
set -euo pipefail
D="$(cd "$(dirname "$0")" && pwd)"
BRIDGE="$(cd "$D/../../../rfirma-native-bridge" && pwd)"

mvn -q -f "$BRIDGE/pom.xml" package -DskipTests
mvn -q -f "$BRIDGE/pom.xml" dependency:build-classpath -Dmdep.outputFile="$BRIDGE/target/cp.txt"
cp "$BRIDGE/target/rfirma-native-bridge-0.1.0.jar" "$BRIDGE/target/cp.txt" "$D/"

# Certificado de usar y tirar: aqui solo se mide donde cae el recuadro, no la
# confianza de la cadena. El de la FNMT no aporta nada a esta pregunta.
openssl req -x509 -newkey rsa:2048 -nodes -days 3650 \
    -subj "/CN=Prueba rfirma" -keyout "$D/key.pem" -out "$D/cert.pem" 2>/dev/null
openssl x509 -in "$D/cert.pem" -outform DER | base64 -w0 > "$D/cert.b64"
rm -f "$D/cert.pem"
cp "$BRIDGE/testbench/inject-pk1.py" "$D/"
echo "motor listo en $D"
