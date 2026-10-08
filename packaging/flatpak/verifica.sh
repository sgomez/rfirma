#!/usr/bin/env bash
# Prueba de humo manual del flatpak: construye, instala y mide lo que la verificacion de paquetes no ve.
#
# El ciclo trifasico corre en el anfitrion contra la libreria instalada en el bundle, no dentro del
# sandbox: alli no hay token PKCS#11, ni poppler, ni forma de invocar los comandos sin el WebView.
#
# Uso: just flatpak-smoke
# Requisitos: flatpak-builder, `just native`, `just build-ts`, `just certs install` y poppler-utils.
set -uo pipefail

AQUI="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RAIZ="$(cd "$AQUI/../.." && pwd)"
APP=me.sgomez.rfirma
LAB=${LAB:-/tmp/rfirma-verifica}
rm -rf "$LAB"; mkdir -p "$LAB"

EXTRA=(--filesystem="$LAB")

RAIZ_NATIVA="$RAIZ/rfirma-native-bridge/target/lib/rfirma/librfirma_crypto.so"
if [ ! -f "$RAIZ_NATIVA" ]; then
    echo "falta la libreria nativa en $RAIZ_NATIVA; ejecuta 'just native'"; exit 1
fi
if [ ! -d "$RAIZ/rfirma-app/dist" ]; then
    echo "falta rfirma-app/dist; ejecuta 'just build-ts'"; exit 1
fi

echo "### 1. construccion"
flatpak-builder --user --force-clean --install --repo="$LAB/repo" \
    "$AQUI/build-dir" "$AQUI/$APP.yml" >"$LAB/build.log" 2>&1 \
    || { echo "FALLO la construccion, ver $LAB/build.log"; exit 1; }
echo "OK  ($(du -sh "$AQUI/build-dir/files" | cut -f1) instalados)"

echo
echo "### 1b. el bundle declara el socket de PC/SC"
if ! flatpak info --show-permissions "$APP" | grep -q '^sockets=.*\bpcsc\b'; then
    echo "EL BUNDLE NO DECLARA --socket=pcsc: sin el socket del pcscd del" >&2
    echo "anfitrion el flatpak no ve lectores ni el DNIe (ADR-0049)." >&2
    exit 1
fi
echo "--socket=pcsc declarado: OK"

echo
echo "### 2. la ventana arranca (WebKitGTK del runtime)"
# Un proceso vivo no es una ventana que se vea: el bus de sesion delata la pagina que no cargo.
flatpak run --log-session-bus --filesystem="$LAB" "$APP" >"$LAB/gui.log" 2>&1 &
sleep 10
if ! pgrep -x rfirma >/dev/null; then
    echo "LA VENTANA MURIO:"; tail -3 "$LAB/gui.log"; exit 1
fi
echo "sigue viva a los 10 s: OK"
flatpak kill "$APP" 2>/dev/null
rechazos=$(grep -c "portal.Error.NotAllowed" "$LAB/gui.log" || true)
if [ "$rechazos" != "0" ]; then
    echo "EL SANDBOX RECHAZA $rechazos LLAMADA(S) AL PORTAL:" >&2
    grep -B1 "portal.Error.NotAllowed" "$LAB/gui.log" >&2
    echo >&2
    echo "Una ventana viva NO es una ventana que se vea: si el rechazo es a" >&2
    echo "ProxyResolver.Lookup, WebKit pinta el error como pagina y no hay" >&2
    echo "aplicacion. Se corrige con --env=GIO_USE_PROXY_RESOLVER=dummy en" >&2
    echo "finish-args, NO con --share=network." >&2
    exit 1
fi
echo "ninguna llamada al portal rechazada: OK"

echo
echo "### 3. el ciclo trifasico completo, contra la libreria DEL BUNDLE"
DESPLIEGUE="$(flatpak info --show-location "$APP" 2>/dev/null)" \
    || { echo "no encuentro el despliegue de $APP"; exit 1; }
LIB_BUNDLE="$DESPLIEGUE/files/lib/rfirma"
if [ ! -f "$LIB_BUNDLE/librfirma_crypto.so" ]; then
    echo "no esta librfirma_crypto.so en $LIB_BUNDLE"; exit 1
fi
echo "libreria: $LIB_BUNDLE/librfirma_crypto.so"
echo "          $(sha256sum "$LIB_BUNDLE/librfirma_crypto.so" | cut -c1-16)... \
($(du -h "$LIB_BUNDLE/librfirma_crypto.so" | cut -f1))"

(
    cd "$RAIZ/rfirma-app/src-tauri" \
        && RFIRMA_LIB_DIR="$LIB_BUNDLE" \
           cargo test --all-features \
           --test native_cycle --test native_cycle_cades --test native_cycle_xades \
           --test native_cycle_visual --test native_cycle_seal -- \
           --include-ignored full_cycle::
) || { echo "FALLO el ciclo contra la libreria del bundle"; exit 1; }
echo "OK  ciclo trifasico y pdfsig contra los bytes que se distribuyen"

echo
echo "### 4. el portal de documentos, dentro del sandbox"
ENTRADA="$LAB/portal-entrada.pdf"
head -c 65536 /dev/urandom >"$ENTRADA"
HASH_ANFITRION=$(sha256sum "$ENTRADA" | cut -d' ' -f1)
RUTA_PORTAL=$(flatpak document-export --app="$APP" "$ENTRADA") \
    || { echo "FALLO 'flatpak document-export'"; exit 1; }
echo "ruta que ve la aplicacion: $RUTA_PORTAL"
HASH_SANDBOX=$(flatpak run "${EXTRA[@]}" --command=sha256sum "$APP" "$RUTA_PORTAL" | cut -d' ' -f1) \
    || { echo "NO HE PODIDO LEER $RUTA_PORTAL DENTRO DEL SANDBOX"; exit 1; }
[ "$HASH_SANDBOX" = "$HASH_ANFITRION" ] \
    && echo "OK  los bytes del portal llegan intactos ($HASH_SANDBOX)" \
    || { echo "LOS BYTES NO COINCIDEN: anfitrion $HASH_ANFITRION, sandbox $HASH_SANDBOX"; exit 1; }

# El permiso del portal sobrevive a cerrar y reabrir la aplicacion.
flatpak run --filesystem="$LAB" "$APP" >"$LAB/sesion-portal.log" 2>&1 &
for _ in $(seq 20); do pgrep -x rfirma >/dev/null && break; sleep 1; done
pgrep -x rfirma >/dev/null \
    || { echo "LA SESION DE PRUEBA NO ARRANCO, no se puede medir"; tail -3 "$LAB/sesion-portal.log"; exit 1; }
flatpak kill "$APP" 2>/dev/null
for _ in $(seq 10); do pgrep -x rfirma >/dev/null || break; sleep 1; done
pgrep -x rfirma >/dev/null \
    && { echo "LA SESION DE PRUEBA NO MURIO, el portal no queda medido"; exit 1; }
RUTA_PORTAL_SESION2=$(flatpak document-export --app="$APP" "$ENTRADA") \
    || { echo "FALLO 'flatpak document-export' en la sesion siguiente"; exit 1; }
if [ "$RUTA_PORTAL_SESION2" != "$RUTA_PORTAL" ]; then
    echo "EL IDENTIFICADOR NO SOBREVIVE: la sesion siguiente recibe otra ruta"
    echo "    ($RUTA_PORTAL -> $RUTA_PORTAL_SESION2)"
    echo "    -> los recientes NO podran reabrir por el portal sin volver a" \
         "pedir permiso el dia que se persistan entre sesiones"
    exit 1
fi
HASH_SESION2=$(flatpak run "${EXTRA[@]}" --command=sha256sum "$APP" "$RUTA_PORTAL_SESION2" | cut -d' ' -f1) \
    || { echo "NO HE PODIDO RELEER $RUTA_PORTAL_SESION2 en la sesion siguiente"; exit 1; }
[ "$HASH_SESION2" = "$HASH_ANFITRION" ] \
    && echo "OK  el identificador del portal sobrevive a cerrar y reabrir la aplicacion" \
    || { echo "EL IDENTIFICADOR SOBREVIVE PERO LOS BYTES NO COINCIDEN"; exit 1; }
echo "    -> el dia que los recientes persistan entre sesiones, reabrir por" \
     "el portal el mismo host path funcionara sin pedir permiso otra vez"

echo
echo "### 5. el sandbox SI puede escribir en los almacenes NSS"
# La comprobacion se decide con un `test -d` dentro del sandbox, nunca leyendo un mensaje de error localizado.
comprueba_escritura() {
    local etiqueta="$1" ruta="$2"
    if [ ! -e "$ruta" ]; then
        echo "AVISO  $etiqueta: no existe $ruta en el anfitrion, nada que medir"
        return 0
    fi
    local salida
    salida=$(flatpak run "${EXTRA[@]}" --env=RUTA_NSS="$ruta" --command=sh "$APP" -c \
        '[ -d "$RUTA_NSS" ] && echo MONTADO || echo NO_MONTADO
         touch "$RUTA_NSS/verifica-escritura-344" 2>&1; echo RC=$?
         rm -f "$RUTA_NSS/verifica-escritura-344"') \
        || { echo "NO HE PODIDO EJECUTAR LA COMPROBACION EN $etiqueta"; exit 1; }
    if ! echo "$salida" | grep -q "^MONTADO$"; then
        echo "$etiqueta NO ESTA MONTADO dentro del sandbox (existe en el anfitrion"
        echo "en $ruta pero dentro no hay directorio): falta el permiso del manifiesto"
        echo "$salida"
        exit 1
    fi
    if ! echo "$salida" | grep -q "RC=0"; then
        echo "ESCRITURA DENEGADA en $etiqueta dentro del sandbox: la CA local no"
        echo "puede entrar y la sede no completara el saludo TLS"
        echo "$salida"
        exit 1
    fi
    echo "OK  $etiqueta: escritura permitida dentro del sandbox"
    echo "    $salida"
}
comprueba_escritura "perfil de Firefox" "$HOME/.mozilla/firefox"
comprueba_escritura "almacen NSS del sistema" "$HOME/.pki/nssdb"

if [ -f "$HOME/.mozilla/firefox/profiles.ini" ]; then
    flatpak run "${EXTRA[@]}" --command=cat "$APP" "$HOME/.mozilla/firefox/profiles.ini" >/dev/null \
        && echo "OK  profiles.ini se lee dentro del sandbox (la otra mitad del permiso)" \
        || { echo "NO HE PODIDO LEER profiles.ini dentro del sandbox"; exit 1; }
else
    echo "AVISO  no hay profiles.ini en el anfitrion, la lectura no se mide"
fi
