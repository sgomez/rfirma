#!/usr/bin/env bash
# Las invariantes de los workflows que ninguna ejecucion detecta: que ninguna
# accion entra por etiqueta (ID-170), que el workflow de construccion no ve un
# secreto jamas (ID-167), y las tres que sostienen la tuberia de entrega
# —nadie hereda secretos en bloque, la Release nace en borrador (ID-168) y una
# candidata no llega a ningun repositorio—.
#
# ------------------------------------------------------------------ ID-170 --
# NINGUNA accion de terceros entra por etiqueta.
#
# Una etiqueta (`@v4`, `@main`) es un puntero que su propietario puede mover
# cuando quiera, y moverlo es ejecutar codigo nuevo dentro de un runner que ya
# tiene el token del repositorio. Un SHA no se mueve. Lo que se compra con esto
# no es inmunidad —el SHA fijado puede ser malo desde el primer dia— sino que
# la actualizacion pase por una PR de Dependabot que alguien mira, en vez de
# ocurrir en silencio (ADR-0015).
#
# Y ES UNA PUERTA Y NO UNA CONVENCION EN UN COMENTARIO porque la convencion se
# rompe sola: quien anada un paso manana copiara el `uses: foo/bar@v1` del
# README de esa accion, que es como estan escritos todos los README del mundo.
#
# Se exige ademas el comentario con la etiqueta al lado, `# v4.4.0`: sin el,
# nadie sabe que version es ese SHA sin abrir un navegador, y Dependabot lo
# necesita para saber desde donde actualiza.
#
# Uso: .github/check-workflows.sh
set -euo pipefail

raiz="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$raiz"

fallos=0

while IFS= read -r linea; do
    fichero="${linea%%:*}"
    resto="${linea#*:}"
    numero="${resto%%:*}"
    texto="${resto#*:}"

    # Una accion local (`uses: ./.github/actions/...`) no viene de nadie de
    # fuera: es este mismo arbol, ya fijado por el commit que se comprueba.
    case "$texto" in
        *uses:*./*) continue ;;
    esac

    # `owner/repo@<40 hex>` u `owner/repo/ruta@<40 hex>`, con el comentario de
    # la etiqueta detras. La comilla opcional cubre el estilo `uses: "..."`.
    if [[ "$texto" =~ uses:[[:space:]]*\"?[A-Za-z0-9._-]+/[A-Za-z0-9._/-]+@[0-9a-f]{40}\"?[[:space:]]*\#[[:space:]]*[^[:space:]]+ ]]; then
        continue
    fi

    echo "$fichero:$numero: accion sin fijar por SHA (ID-170)" >&2
    echo "    ${texto# }" >&2
    fallos=$((fallos + 1))
done < <(grep -rn '^[[:space:]]*-\?[[:space:]]*uses:' .github/workflows || true)

if [ "$fallos" -ne 0 ]; then
    echo >&2
    echo "$fallos accion(es) sin fijar. Cada una se escribe asi:" >&2
    echo "    uses: actions/checkout@11d5960a326750d5838078e36cf38b85af677262 # v4.4.0" >&2
    echo "El SHA se saca con: gh api repos/<owner>/<repo>/commits/<etiqueta> --jq .sha" >&2
    exit 1
fi

echo "OK  todas las acciones de .github/workflows estan fijadas por SHA"

# ------------------------------------------------------------------ ID-167 --
# El workflow de construccion NO VE UN SECRETO JAMAS.
#
# Es la primera invariante del ADR-0015 y la que sostiene todo lo demas:
# `build.yml` es invocable, asi que si viese un secreto, reutilizarlo desde
# cualquier sitio seria un camino hacia la subclave de firma que vive en el
# entorno de release. Lo que se prohibe es NOMBRAR el contexto `secrets` —donde
# una accion pide un token se le da `github.token`, que es el mismo valor sin
# obligar al llamador a pasar nada— y declarar `secrets:` en el `workflow_call`.
BUILD=.github/workflows/build.yml
if [ -f "$BUILD" ]; then
    # Las lineas de comentario se descartan: la cabecera del propio fichero
    # explica la invariante, y para explicarla tiene que nombrarla.
    menciones="$(grep -nE 'secrets[.[]|^[[:space:]]*secrets:' "$BUILD" \
        | grep -vE '^[0-9]+:[[:space:]]*#' || true)"
    if [ -n "$menciones" ]; then
        printf '%s\n' "$menciones" >&2
        echo >&2
        echo "$BUILD no puede mencionar ningun secreto (ID-167, ADR-0015)." >&2
        echo "Para un token de la propia ejecucion, usa \${{ github.token }}." >&2
        exit 1
    fi
    if ! grep -q '^  workflow_call:' "$BUILD"; then
        echo "$BUILD tiene que seguir siendo invocable (workflow_call, ID-167)" >&2
        exit 1
    fi
    # Solo lectura, y en el nivel del workflow: declararlo por job dejaria al
    # siguiente job que alguien anada con los permisos por omision.
    if ! awk '
        /^permissions:$/ { dentro = 1; next }
        dentro && /^  contents: read$/ { ok = 1 }
        dentro && /^[^ ]/ { dentro = 0 }
        END { exit ok ? 0 : 1 }
    ' "$BUILD"; then
        echo "$BUILD tiene que declarar 'permissions: contents: read' (ID-167)" >&2
        exit 1
    fi
    echo "OK  $BUILD es invocable, de solo lectura y no menciona ningun secreto"
fi

# ------------------------------------------------- ID-167, ID-168, ID-169 --
# LAS INVARIANTES DE LOS OTROS DOS WORKFLOWS.
#
# La cabecera de build.yml deja dicho lo que ese fichero NO puede impedir por
# si mismo: que un llamador escriba `secrets: inherit` y le entregue de golpe
# todo el entorno de release, subclave GPG incluida. Eso se «vigila al revisar
# release.yml», y una vigilancia que depende de que alguien se acuerde no es
# una vigilancia. Aqui esta escrita.
#
# Las otras dos son del mismo tamano: la Release NACE EN BORRADOR (ID-168) y el
# despliegue NO OCURRE PARA UNA PRERELEASE (ADR-0015). Las dos se pierden
# borrando una sola linea, y ninguna ejecucion del CI las echaria de menos:
# solo se notarian el dia de la entrega, publicando algo que nadie ha mirado.
# Las lineas de comentario quedan fuera, igual que arriba: las cabeceras de
# build.yml y de release.yml explican la prohibicion, y para explicarla tienen
# que escribirla.
herencias="$(grep -rn 'secrets:[[:space:]]*inherit' .github/workflows \
    | grep -vE ':[0-9]+:[[:space:]]*#' || true)"
if [ -n "$herencias" ]; then
    printf '%s\n' "$herencias" >&2
    echo >&2
    echo "ningun workflow hereda secretos en bloque (ID-167, ADR-0015)." >&2
    echo "Heredarlos sobre build.yml le entrega el entorno de release entero." >&2
    echo "Pasa uno a uno los que el workflow llamado declare." >&2
    exit 1
fi
echo "OK  ningun workflow escribe 'secrets: inherit'"

RELEASE=.github/workflows/release.yml
if [ -f "$RELEASE" ]; then
    if ! grep -q '^    environment: release$' "$RELEASE"; then
        echo "$RELEASE tiene que firmar dentro de 'environment: release' (ADR-0015)" >&2
        exit 1
    fi
    if ! grep -q -- '--draft' "$RELEASE"; then
        echo "$RELEASE tiene que crear la Release en borrador (ID-168)." >&2
        echo "Publicarla es el gesto humano; una etiqueta no publica nada." >&2
        exit 1
    fi
    echo "OK  $RELEASE firma en el entorno de release y deja el borrador"
fi

PUBLISH=.github/workflows/publish.yml
if [ -f "$PUBLISH" ]; then
    if ! grep -q 'types: \[published\]' "$PUBLISH"; then
        echo "$PUBLISH cuelga de 'release: types: [published]', no de la etiqueta (ID-167)" >&2
        exit 1
    fi
    if ! grep -q 'github.event.release.prerelease' "$PUBLISH"; then
        echo "$PUBLISH tiene que descartar las prereleases (ADR-0015)." >&2
        echo "Una etiqueta -rc.N ensaya la tuberia y no llega a ningun repositorio." >&2
        exit 1
    fi
    echo "OK  $PUBLISH solo reacciona a una Release publicada que no es candidata"

    # EL MECANISMO DE LA PUBLICACION VIVE EN UN GUION QUE SE PRUEBA (ID-174).
    # Es la unica parte de la tuberia que no se puede ensayar con una etiqueta
    # `-rc.N` —el ensayo se detiene antes de tocar el anfitrion—, asi que un
    # `rsync` escrito a mano dentro del workflow seria una orden que nadie ha
    # ejecutado nunca hasta el dia de la entrega. Y el orden de las tres
    # ordenes (arbol, enlace, poda) es lo que hace que un despliegue a medias
    # no se vea: si se reparte entre pasos del YAML, deja de estar probado.
    if ! grep -q 'packaging/repo/publish-tree.sh' "$PUBLISH"; then
        echo "$PUBLISH tiene que publicar con packaging/repo/publish-tree.sh (ID-174)" >&2
        exit 1
    fi
    sueltos="$(grep -nE '^[[:space:]]+(-[[:space:]]+)?(run:[[:space:]]*)?rsync ' "$PUBLISH" || true)"
    if [ -n "$sueltos" ]; then
        printf '%s\n' "$sueltos" >&2
        echo >&2
        echo "el rsync de la publicacion va en packaging/repo/publish-tree.sh, no aqui." >&2
        echo "Ahi esta probado (just check-publish); en el YAML no lo prueba nadie." >&2
        exit 1
    fi
    echo "OK  $PUBLISH publica con el guion probado y no lleva rsync suelto"
fi

# ------------------------------------------------------------------ ID-174 --
# NI REGISTRO DE IMAGENES NI UN SOLO PASO DE DOCKER EN EL CI.
#
# Los repositorios no van dentro de la imagen —una imagen no es sitio para
# datos que crecen: cada publicacion produciria una capa nueva con la historia
# entera repetida—, y con los datos fuera la tuberia no toca Docker en ningun
# momento. La imagen es solo el servidor web con la landing y la construye
# Coolify desde `main`.
#
# Se vigila aqui porque la tentacion tiene nombre y viene sola: el dia que
# alguien quiera «probar la imagen en el CI» anadira un `docker build`, y
# detras de un `docker build` viene un registro, y detras de un registro
# vienen los datos dentro de la imagen. Los comentarios quedan fuera: esta
# prohibicion hay que poder explicarla nombrandola.
docker="$(grep -rniE 'docker|ghcr\.io|container-registry' .github/workflows \
    | grep -vE ':[0-9]+:[[:space:]]*#' || true)"
if [ -n "$docker" ]; then
    printf '%s\n' "$docker" >&2
    echo >&2
    echo "ningun workflow toca Docker ni un registro de imagenes (ID-174, ADR-0015)." >&2
    echo "Los tres repositorios llegan al anfitrion por rsync, fuera de la imagen." >&2
    exit 1
fi
echo "OK  ningun workflow toca Docker ni un registro de imagenes"
