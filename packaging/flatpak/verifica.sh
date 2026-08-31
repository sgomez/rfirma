#!/usr/bin/env bash
# Verificacion reproducible del flatpak. Construye, instala y comprueba dentro
# del arenero: que la libreria nativa esta donde el ADR-0004 dice, que el modulo
# PKCS#11 que empaqueta el propio flatpak carga, y que la ventana arranca y
# sigue viva.
#
# QUE SE MIDE AQUI Y QUE NO. Hasta el #56 este script pedia a la sonda del #22
# que ejecutase el ciclo trifasico entero dentro del arenero y validaba el PDF
# con pdfsig. La sonda se borro con el ADR-0013 —era una segunda implementacion
# de la frontera FFI—, y rfirma todavia no orquesta las tres fases: eso lo
# aporta un sub-issue posterior del #46. Mientras tanto la firma de punta a
# punta la mide `just test-native`, FUERA del arenero, y lo que se comprueba
# aqui es lo que solo el arenero puede romper.
#
# PENDIENTE: recuperar los pasos de firma cuando exista la orquestacion. Lo que
# median —que dentro del arenero se puede cargar la libreria, hablar con pcscd y
# leer el modulo PKCS#11 empaquetado— esta escrito en
# docs/research/flatpak-canal-unico.md.
#
# Uso: packaging/flatpak/verifica.sh
#
# Requisitos: flatpak-builder, org.gnome.Sdk//50,
#             org.freedesktop.Sdk.Extension.rust-stable//25.08,
#             la libreria nativa en la ruta canonica del ADR-0013
#             (GRAALVM_HOME=CE 25; `just native`) y el frontend construido
#             (`just build-ts`, que tauri-build lee de rfirma-app/dist).
set -uo pipefail

AQUI="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RAIZ="$(cd "$AQUI/../.." && pwd)"
APP=me.sgomez.rfirma
LAB=${LAB:-/tmp/rfirma-verifica}
rm -rf "$LAB"; mkdir -p "$LAB"

# El unico permiso extra del banco de pruebas es el laboratorio donde cae lo
# que se inspecciona. En produccion NO se pasa ninguno: el modulo PKCS#11 lo
# empaqueta el propio flatpak y los ficheros entran por portales.
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
echo "### 2. la libreria nativa, dentro del arenero"
# UN SOLO FICHERO (ADR-0012). Si aqui aparece libawt.so, un JPEG con perfil ICC
# aborta el proceso en vez de dar un error recuperable: eso es un fallo, no un
# extra.
flatpak run "${EXTRA[@]}" --command=ls "$APP" -1 /app/lib/rfirma
contenido=$(flatpak run --command=ls "$APP" -1 /app/lib/rfirma)
[ "$contenido" = "librfirma_crypto.so" ] \
    && echo "OK  un solo fichero" \
    || { echo "SOBRA ALGO en /app/lib/rfirma"; exit 1; }

echo
echo "### 3. el modulo PKCS#11 que empaqueta el flatpak"
# El del anfitrion no carga aqui dentro (medido en #22): sus libopensc.so.13 y
# libeac.so.3 no estan.
flatpak run "${EXTRA[@]}" --command=sh "$APP" -c \
    'ls -1 /app/lib/pkcs11/opensc-pkcs11.so && ls -1 /app/lib/libpcsclite.so.1' \
    && echo "OK  modulo y cliente PC/SC presentes" \
    || { echo "FALTA el modulo PKCS#11 o el cliente PC/SC"; exit 1; }

echo
echo "### 4. la ventana arranca (WebKitGTK del runtime)"
flatpak run --filesystem="$LAB" "$APP" >"$LAB/gui.log" 2>&1 &
sleep 10
if pgrep -x rfirma >/dev/null; then
    # rfirma no escribe nada en el arranque: lo que se mide es que el proceso
    # sobreviva, porque el fallo del #22 era que Mutter lo mataba (Error 71).
    echo "sigue viva a los 10 s: OK"
    flatpak kill "$APP" 2>/dev/null
else
    echo "LA VENTANA MURIO:"; tail -3 "$LAB/gui.log"; exit 1
fi

echo
echo "### 5. bundle de un solo fichero"
flatpak build-bundle "$LAB/repo" "$LAB/$APP.flatpak" "$APP" stable >/dev/null 2>&1 \
    && echo "$LAB/$APP.flatpak: $(du -h "$LAB/$APP.flatpak" | cut -f1)" \
    || echo "build-bundle FALLO"
