#!/usr/bin/env bash
# Verificacion reproducible del flatpak. Construye, instala y comprueba dentro
# del sandbox: que el modulo PKCS#11 que empaqueta el propio flatpak carga, y
# que la ventana arranca y sigue viva. La invariante del ADR-0012 —un solo
# librfirma_crypto.so, libawt.so en ninguna parte— ya NO vive aqui: se comprueba
# en packaging/verifica-contenido.sh, independiente del formato (ID-144).
#
# QUE SE MIDE AQUI Y QUE NO.
#
# El paso 4 corre el CICLO TRIFASICO COMPLETO con RUBRICA DE IMAGEN contra la
# libreria que ha quedado INSTALADA dentro del bundle —los bytes que se
# distribuyen, sacados de /app/lib/rfirma— y valida el PDF con pdfsig. Eso es lo
# que faltaba: la verificacion del #22 se corrio contra la imagen de SEIS
# ficheros (ce25-awt), y la rubrica de imagen es justo el caso que cambia de
# comportamiento segun que .so haya al lado. Con libawt.so presente, un JPEG con
# perfil ICC aborta el proceso (rc=134) en vez de dar un error recuperable.
#
# El ciclo se ejecuta en el ANFITRION apuntando a la libreria del bundle, no
# dentro del sandbox. No es una preferencia; dentro no se puede hoy, por tres
# razones medidas:
#
#   1. NO HAY TOKEN. La fase 2 firma con PKCS#11, y el bundle empaqueta OpenSC
#      para una tarjeta fisica (ID-40). El token de pruebas es SoftHSM, que no
#      esta dentro y no puede entrar: montar el del anfitrion es apuntar
#      LD_LIBRARY_PATH a librerias de otra glibc, que es exactamente lo que el
#      ID-40 prohibe.
#   2. NO HAY POPPLER. `pdfsig` y `pdftoppm` no estan ni en el bundle ni en
#      org.gnome.Platform//50 (comprobado: `which pdfsig` no los encuentra), y
#      son la puerta automatica de la TD-03.
#   3. NO HAY POR DONDE ENTRAR. rfirma no tiene modo headless: el ciclo solo se
#      alcanza por los #[tauri::command] desde el WebView. Y un binario de
#      prueba construido en el anfitrion tampoco vale de puente, porque aqui la
#      glibc es 2.43 y la del runtime 2.42.
#
# Meter SoftHSM, poppler y un binario de prueba en el bundle para poder medirlo
# desde dentro seria distribuir el banco de pruebas a las personas usuarias y
# romper "los permisos son los declarados y ninguno mas". Cerrar el hueco pide
# un manifiesto de banco aparte, y eso es otra decision.
#
# Lo que SI se mide dentro del sandbox son los pasos 2, 3, 5 y 6: que el
# modulo PKCS#11 que empaqueta el propio flatpak carga, que la ventana arranca
# y sigue viva, que un documento entrado por el portal llega con sus bytes
# intactos, y que el sandbox RECHAZA escribir en los almacenes NSS que el
# manifiesto declara en solo lectura (#101, AC 3). El paso 5 no necesita el
# WebView para comprobar el portal: usa `flatpak document-export`, la misma
# via por la que el dialogo de abrir concede el permiso
# (docs/research/flatpak-canal-unico.md, apartado 4), asi que no hace falta el
# binario de prueba que el paso 4 SI necesita para el ciclo trifasico. Ver
# docs/research/flatpak-canal-unico.md.
#
# Uso: packaging/flatpak/verifica.sh
#
# Requisitos: flatpak-builder, org.gnome.Sdk//50,
#             org.freedesktop.Sdk.Extension.rust-stable//25.08,
#             la libreria nativa en la ruta canonica del ADR-0013
#             (GRAALVM_HOME=CE 25; `just native`) y el frontend construido
#             (`just build-ts`, que tauri-build lee de rfirma-app/dist).
#             Para el paso 4, ademas: el token SoftHSM de la grada B
#             (`just token`) y poppler-utils (`pdfsig`, `pdftoppm`).
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
echo "### 2. el modulo PKCS#11 que empaqueta el flatpak"
# El del anfitrion no carga aqui dentro (medido en #22): sus libopensc.so.13 y
# libeac.so.3 no estan.
flatpak run "${EXTRA[@]}" --command=sh "$APP" -c \
    'ls -1 /app/lib/pkcs11/opensc-pkcs11.so && ls -1 /app/lib/libpcsclite.so.1' \
    && echo "OK  modulo y cliente PC/SC presentes" \
    || { echo "FALTA el modulo PKCS#11 o el cliente PC/SC"; exit 1; }

echo
echo "### 3. la ventana arranca (WebKitGTK del runtime)"
# --log-session-bus NO es adorno: un proceso VIVO no es una ventana que se vea.
# El v0.1 se entrego con la ventana pintando un error de DBus a pantalla
# completa y este paso decia OK, porque solo miraba el pgrep. rfirma no escribe
# nada en el arranque, asi que lo unico que delata una pagina que no ha cargado
# es lo que el sandbox rechaza en el bus.
flatpak run --log-session-bus --filesystem="$LAB" "$APP" >"$LAB/gui.log" 2>&1 &
sleep 10
if ! pgrep -x rfirma >/dev/null; then
    echo "LA VENTANA MURIO:"; tail -3 "$LAB/gui.log"; exit 1
fi
# El fallo del #22 era que Mutter la mataba (Error 71); el de la v0.1, que
# WebKitGTK pedia un proxy que el sandbox sin red no concede.
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
echo "### 4. el ciclo trifasico completo, contra la libreria DEL BUNDLE"
# Los bytes que se distribuyen, no los del arbol de construccion: se leen del
# despliegue que acaba de instalar el paso 1. Si esto y `just test-native`
# midiesen el mismo fichero, este paso no diria nada nuevo.
DESPLIEGUE="$(flatpak info --show-location "$APP" 2>/dev/null)" \
    || { echo "no encuentro el despliegue de $APP"; exit 1; }
LIB_BUNDLE="$DESPLIEGUE/files/lib/rfirma"
if [ ! -f "$LIB_BUNDLE/librfirma_crypto.so" ]; then
    echo "no esta librfirma_crypto.so en $LIB_BUNDLE"; exit 1
fi
echo "libreria: $LIB_BUNDLE/librfirma_crypto.so"
echo "          $(sha256sum "$LIB_BUNDLE/librfirma_crypto.so" | cut -c1-16)... \
($(du -h "$LIB_BUNDLE/librfirma_crypto.so" | cut -f1))"

# La rubrica de imagen es el caso que este paso existe para cubrir, asi que se
# ejecutan los cuatro casos visibles y la cofirma: el filtro es el modulo entero
# full_cycle. Las pruebas generan el JPEG con el normalizador de produccion
# (ADR-0012), validan con pdfsig y comprueban la rubrica RASTERIZANDO, porque
# pdftotext no la ve y daria un falso negativo (TD-03).
(
    cd "$RAIZ/rfirma-app/src-tauri" \
        && RFIRMA_LIB_DIR="$LIB_BUNDLE" \
           cargo test --all-features --test native_cycle -- \
           --include-ignored full_cycle::
) || { echo "FALLO el ciclo contra la libreria del bundle"; exit 1; }
echo "OK  ciclo trifasico y pdfsig contra los bytes que se distribuyen"

echo
echo "### 5. el portal de documentos, dentro del sandbox"
# DENTRO del sandbox: `flatpak run` hereda exactamente los mismos permisos
# que el binario real, y `flatpak document-export` es la misma via por la que
# el dialogo de abrir (Orden 7, commands/mod.rs) concede el permiso -contra la
# RUTA del anfitrion, no el inodo- y devuelve la ruta montada que la
# aplicacion ve (docs/research/flatpak-canal-unico.md, apartado 4). No hace
# falta el binario de prueba que el paso 4 necesita: aqui no se firma nada,
# solo se comprueba que los bytes concedidos por el portal llegan, que es lo
# que la Orden 8 (`read_document`) lee del disco.
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

# ID-72: si el permiso sobrevive a cerrar y reabrir la aplicacion, y no solo a
# seguir vivo dentro del mismo `flatpak run`. Se simula la sesion siguiente
# matando el proceso y volviendo a pedir la MISMA ruta del anfitrion, como
# haria quien vuelve a elegir el mismo fichero en el dialogo.
flatpak run --filesystem="$LAB" "$APP" >"$LAB/sesion-portal.log" 2>&1 &
for _ in $(seq 20); do pgrep -x rfirma >/dev/null && break; sleep 1; done
pgrep -x rfirma >/dev/null \
    || { echo "LA SESION DE PRUEBA NO ARRANCO, no se puede medir el ID-72"; tail -3 "$LAB/sesion-portal.log"; exit 1; }
flatpak kill "$APP" 2>/dev/null
for _ in $(seq 10); do pgrep -x rfirma >/dev/null || break; sleep 1; done
pgrep -x rfirma >/dev/null \
    && { echo "LA SESION DE PRUEBA NO MURIO, el ID-72 no queda medido"; exit 1; }
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
echo "### 6. el sandbox no puede escribir en los almacenes NSS (:ro, #101 AC 3)"
# AC 3 del #101: "rfirma no puede escribir en el perfil de Firefox desde
# dentro del sandbox, y hay constancia de haberlo comprobado". Esto NO es la
# comprobacion que la TD del #95 deja como manual -esa depende de una maquina
# y un perfil reales-; esta es una propiedad del sandbox, medible sin ninguno
# de los dos: :ro tiene que sostenerse tanto si hay perfil como si no.
#
# TRAMPA MEDIDA (misma clase que xdg-documents, #27): si la ruta NO existe en
# el ANFITRION, flatpak no monta nada y el intento de escritura falla por
# ENOENT igual que si :ro estuviese haciendo su trabajo -un "OK" que no ha
# medido nada. Por eso la funcion distingue tres desenlaces: la escritura se
# permitio (FALLO, :ro no se sostiene), la ruta no esta montada pese a existir
# en el anfitrion (FALLO, no se ha medido lo que se creia medir) y no hay nada
# que montar porque el anfitrion no tiene esa ruta (AVISO, nada que medir).
#
# NINGUNO de los tres desenlaces se decide leyendo un mensaje de error. Un
# `grep` a la salida de `touch` solo acierta con coreutils en ingles: `flatpak
# run` propaga LANG/LC_* del anfitrion al sandbox y org.gnome.Platform//50 trae
# la extension de locale, asi que en un escritorio en castellano el mensaje es
# "No existe el fichero o el directorio" y la rama de "no esta montado" no se
# dispararia -otro falso verde, justo el que esta funcion cierra-. Asi que el
# montaje se decide con un `test -d` DENTRO del sandbox, que es la pregunta de
# verdad, y el mensaje de `touch` queda como informacion para quien lea la
# salida. La ruta viaja por RUTA_RO en el entorno y no interpolada dentro de las
# comillas simples de `sh -c`, para que un $HOME con comillas no rompa la orden.
comprueba_ro() {
    local etiqueta="$1" ruta="$2"
    if [ ! -e "$ruta" ]; then
        echo "AVISO  $etiqueta: no existe $ruta en el anfitrion, nada que medir"
        return 0
    fi
    local salida
    salida=$(flatpak run "${EXTRA[@]}" --env=RUTA_RO="$ruta" --command=sh "$APP" -c \
        '[ -d "$RUTA_RO" ] && echo MONTADO || echo NO_MONTADO
         touch "$RUTA_RO/verifica-escritura-107" 2>&1; echo RC=$?') \
        || { echo "NO HE PODIDO EJECUTAR LA COMPROBACION EN $etiqueta"; exit 1; }
    if echo "$salida" | grep -q "RC=0"; then
        echo "ESCRITURA PERMITIDA en $etiqueta dentro del sandbox (:ro no se sostiene):"
        echo "$salida"
        flatpak run "${EXTRA[@]}" --env=RUTA_RO="$ruta" --command=sh "$APP" -c \
            'rm -f "$RUTA_RO/verifica-escritura-107"' 2>/dev/null
        exit 1
    fi
    if ! echo "$salida" | grep -q "^MONTADO$"; then
        echo "$etiqueta NO ESTA MONTADO dentro del sandbox (existe en el anfitrion"
        echo "en $ruta pero dentro no hay directorio): no se ha medido nada"
        echo "$salida"
        exit 1
    fi
    echo "OK  $etiqueta: escritura denegada dentro del sandbox"
    echo "    $salida"
}
comprueba_ro "perfil de Firefox" "$HOME/.mozilla/firefox"
comprueba_ro "almacen NSS del sistema" "$HOME/.pki/nssdb"

if [ -f "$HOME/.mozilla/firefox/profiles.ini" ]; then
    flatpak run "${EXTRA[@]}" --command=cat "$APP" "$HOME/.mozilla/firefox/profiles.ini" >/dev/null \
        && echo "OK  profiles.ini se lee dentro del sandbox (la mitad que SI hace falta)" \
        || { echo "NO HE PODIDO LEER profiles.ini dentro del sandbox"; exit 1; }
else
    echo "AVISO  no hay profiles.ini en el anfitrion, la lectura no se mide"
fi

echo
echo "### 7. bundle de un solo fichero"
flatpak build-bundle "$LAB/repo" "$LAB/$APP.flatpak" "$APP" stable >/dev/null 2>&1 \
    && echo "$LAB/$APP.flatpak: $(du -h "$LAB/$APP.flatpak" | cut -f1)" \
    || { echo "build-bundle FALLO"; exit 1; }

echo
echo "### 8. la invariante del ADR-0012, sobre el bundle"
"$(dirname "$AQUI")/verifica-contenido.sh" "$LAB/$APP.flatpak" \
    || { echo "la puerta del contenido ha fallado"; exit 1; }
