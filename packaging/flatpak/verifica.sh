#!/usr/bin/env bash
# Verificacion reproducible del flatpak (#22). Construye, instala y comprueba
# dentro del arenero: los seis .so, el ciclo trifasico completo con rubrica de
# imagen firmado por PKCS#11, la validez de la firma y que la ventana arranca.
#
# Uso: packaging/flatpak/verifica.sh
#
# Requisitos: flatpak-builder, org.gnome.Sdk//50,
#             org.freedesktop.Sdk.Extension.rust-stable//25.08,
#             la imagen nativa en rfirma-native-bridge/target/ce25-awt
#             (GRAALVM_HOME=CE 25; testbench/build-native-awt.sh ce25-awt awt-config),
#             el material de pruebas en target/fixtures (test.pdf,
#             visible-imagen.properties) y el token de pruebas de #5
#             (SoftHSM rfirma-test); cert-fnmt.b64 se exporta solo.
set -uo pipefail

AQUI="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RAIZ="$(cd "$AQUI/../.." && pwd)"
APP=me.sgomez.rfirma
LAB=${LAB:-/tmp/rfirma-verifica}
rm -rf "$LAB"; mkdir -p "$LAB"

# El token de pruebas es de software y vive en el HOME: fuera del arenero. Los
# --filesystem de abajo son del banco de pruebas, NO del manifiesto. En
# produccion el modulo es el opensc que empaqueta el propio flatpak.
P11=/run/host/usr/lib/x86_64-linux-gnu/softhsm/libsofthsm2.so
EXTRA=(
  --filesystem=host-os:ro
  --filesystem="$HOME/.local/share/softhsm"
  --filesystem="$HOME/.config/softhsm2:ro"
  --filesystem="$LAB"
  --env=RFIRMA_P11_MODULE=$P11
  --env=SOFTHSM2_CONF="$HOME/.config/softhsm2/softhsm2.conf"
  --env=RFIRMA_P11_PIN=1234
  --env=RFIRMA_CERT=cert-fnmt.b64
)

# El certificado que se incrusta tiene que ser el DEL TOKEN: firmar con la
# clave del token e incrustar otro certificado da "Signature is Invalid".
FIX="$RAIZ/target/fixtures"
if [ ! -f "$FIX/cert-fnmt.b64" ]; then
    echo "### 0. exportando el certificado del token"
    pkcs11-tool --module /usr/lib/softhsm/libsofthsm2.so --read-object --type cert \
        --label FNMT-ACTIVO-99999999R -o "$FIX/cert-fnmt.der" >/dev/null 2>&1 \
        || { echo "no puedo leer el certificado del token de pruebas (#5)"; exit 1; }
    base64 -w0 "$FIX/cert-fnmt.der" > "$FIX/cert-fnmt.b64"
fi

echo "### 1. construccion"
flatpak-builder --user --force-clean --install --repo="$LAB/repo" \
    "$AQUI/build-dir" "$AQUI/$APP.yml" >"$LAB/build.log" 2>&1 \
    || { echo "FALLO la construccion, ver $LAB/build.log"; exit 1; }
echo "OK  ($(du -sh "$AQUI/build-dir/files" | cut -f1) instalados)"

echo
echo "### 2. entorno del arenero"
flatpak run "${EXTRA[@]}" --command=rfirma-probe "$APP" entorno

echo
echo "### 3. ciclo trifasico completo con rubrica de imagen"
flatpak run "${EXTRA[@]}" --command=rfirma-probe "$APP" \
    ciclo /app/share/rfirma-probe/test.pdf "$LAB/firmado.pdf" 2>&1 \
    | grep -vE "^(INFO|[A-Z][a-z]{2} [0-9])" | grep -E "^(certificado|PDF|dlopen|prefirma|firma|postfirma)"

echo
echo "### 4. validacion (en el anfitrion)"
if [ -f "$LAB/firmado.pdf" ]; then
    pdfsig "$LAB/firmado.pdf" 2>&1 | grep -E "Signature Validation|Signer Certificate Common Name|Signing Time"
    pdftoppm -png -r 50 -f 1 -l 1 "$LAB/firmado.pdf" "$LAB/pag1"
    echo "rasterizado: $(stat -c%s "$LAB"/pag1-01.png) bytes  |  PDF: $(stat -c%s "$LAB/firmado.pdf") bytes"
else
    echo "SIN PDF"; exit 1
fi

echo
echo "### 5. la ventana arranca (WebKitGTK del runtime)"
flatpak run --filesystem="$LAB" "$APP" >"$LAB/gui.log" 2>&1 &
sleep 10
if pgrep -x rfirma-probe >/dev/null; then
    grep -E "^WEBVIEW OK|cargada:" "$LAB/gui.log"
    echo "sigue viva a los 10 s: OK"
    flatpak kill "$APP" 2>/dev/null
else
    echo "LA VENTANA MURIO:"; tail -3 "$LAB/gui.log"; exit 1
fi

echo
echo "### 6. bundle de un solo fichero"
flatpak build-bundle "$LAB/repo" "$LAB/$APP.flatpak" "$APP" stable >/dev/null 2>&1 \
    && echo "$LAB/$APP.flatpak: $(du -h "$LAB/$APP.flatpak" | cut -f1)" \
    || echo "build-bundle FALLO"
