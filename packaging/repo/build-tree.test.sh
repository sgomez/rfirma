#!/usr/bin/env bash
# Las comprobaciones de `build-tree.sh`. La que de verdad importa es la de la
# idempotencia: «tirar el volumen y volver a publicar deja el servicio
# identico» solo es cierto mientras nada del arbol dependa del momento en que
# se construyo, y eso es una linea de mas en cualquier momento.
#
# Uso: packaging/repo/build-tree.test.sh
set -euo pipefail

raiz="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
construye="$raiz/packaging/repo/build-tree.sh"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

fallos=0
fail() { echo "FALLO  $*" >&2; fallos=$((fallos + 1)); }
ok() { echo "OK  $*"; }

echo "clave publica de mentira" > "$tmp/rfirma.asc"

version_completa() {
    local dir="$1" version="$2"
    mkdir -p "$dir"
    echo "flatpak $version" > "$dir/rfirma-$version.flatpak"
    echo "deb $version" > "$dir/rfirma_${version}_amd64.deb"
    echo "rpm $version" > "$dir/rfirma-$version.x86_64.rpm"
}

serie="$tmp/serie"
version_completa "$serie/v0.4.0" "0.4.0"
version_completa "$serie/v0.4.10" "0.4.10"
version_completa "$serie/v0.4.2" "0.4.2"

# the_tree_has_the_shape_the_addition_commands_promise
"$construye" "$serie" "$tmp/arbol" "$tmp/rfirma.asc" > "$tmp/salida" 2>&1 \
    || { cat "$tmp/salida" >&2; fail "la construccion falla"; }
if [ -f "$tmp/arbol/rfirma.asc" ] && [ -d "$tmp/arbol/flatpak" ] \
    && [ -d "$tmp/arbol/apt" ] && [ -d "$tmp/arbol/rpm" ]; then
    ok "el arbol tiene la clave y los tres repositorios"
else
    fail "el arbol no tiene la forma que fija el ADR-0015"
fi

# the_versions_are_ordered_by_version_and_not_by_listing
# El orden en que se importan los bundles es la historia del ostree: v0.4.10 va
# DESPUES de v0.4.2, que es justo lo que el orden alfabetico no hace.
if grep -q "v0.4.0 v0.4.2 v0.4.10" "$tmp/salida"; then
    ok "las versiones se ordenan por version, no alfabeticamente"
else
    fail "las versiones no se ordenan por version: $(cat "$tmp/salida")"
fi

# rebuilding_the_same_series_gives_the_same_tree
huella_arbol() { (cd "$1" && find . | LC_ALL=C sort | xargs -I{} sh -c 'printf "%s " "{}"; [ -f "{}" ] && sha256sum < "{}" || echo dir'); }
antes="$(huella_arbol "$tmp/arbol")"
"$construye" "$serie" "$tmp/arbol" "$tmp/rfirma.asc" > /dev/null 2>&1
if [ "$(huella_arbol "$tmp/arbol")" = "$antes" ]; then
    ok "reconstruir la misma serie da el mismo arbol"
else
    fail "reconstruir la misma serie da un arbol distinto"
fi

# a_stale_tree_is_wiped_and_not_merged
echo "de la vez anterior" > "$tmp/arbol/sobra.txt"
"$construye" "$serie" "$tmp/arbol" "$tmp/rfirma.asc" > /dev/null 2>&1
if [ -e "$tmp/arbol/sobra.txt" ]; then
    fail "el arbol anterior se mezcla con el nuevo en vez de rehacerse"
else
    ok "el arbol se rehace entero, no se mezcla con el anterior"
fi

# an_incomplete_series_stops_the_publication
incompleta="$tmp/incompleta"
version_completa "$incompleta/v0.4.0" "0.4.0"
rm "$incompleta/v0.4.0"/*.rpm
if "$construye" "$incompleta" "$tmp/arbol-malo" "$tmp/rfirma.asc" > /dev/null 2>&1; then
    fail "una version sin los tres paquetes no detiene la publicacion"
else
    ok "una version sin los tres paquetes detiene la publicacion"
fi

# an_empty_series_stops_the_publication
mkdir -p "$tmp/vacia"
if "$construye" "$tmp/vacia" "$tmp/arbol-vacio" "$tmp/rfirma.asc" > /dev/null 2>&1; then
    fail "una serie vacia no detiene la publicacion"
else
    ok "una serie vacia detiene la publicacion"
fi

echo
if [ "$fallos" -ne 0 ]; then
    echo "$fallos comprobacion(es) del arbol han fallado" >&2
    exit 1
fi
echo "OK  el arbol se reconstruye entero, ordenado y sin rastro del anterior"
