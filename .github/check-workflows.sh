#!/usr/bin/env bash
# Las dos invariantes de los workflows que ninguna ejecucion detecta: que
# ninguna accion entra por etiqueta (ID-170) y que el workflow de construccion
# no ve un secreto jamas (ID-167).
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
# Uso: .github/check-actions-pinned.sh
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
