#!/usr/bin/env bash
#
# Ejecuta el comando que recibe con su propia copia del almacen de SoftHSM, en un
# directorio temporal con su SOFTHSM2_CONF, que se borra al terminar aunque falle.
# Lo usan nextest (una copia por proceso de prueba) y `just coverage` (una por
# ejecucion). La copia espera al cerrojo `$SOFTHSM2_CONF.lock` de `certs.sh`, para
# no copiar a mitad de una instalacion.

set -euo pipefail

conf="${SOFTHSM2_CONF:-$HOME/.config/softhsm2/softhsm2.conf}"
[ -f "$conf" ] || conf=/etc/softhsm/softhsm2.conf
tokendir="$(sed -n 's/^[[:space:]]*directories\.tokendir[[:space:]]*=[[:space:]]*//p' "$conf" 2>/dev/null || true)"
if [ -z "$tokendir" ] || [ ! -d "$tokendir" ]; then
    exec "$@"
fi

copy="$(mktemp -d "${TMPDIR:-/tmp}/rfirma-token.XXXXXX")"
trap 'rm -rf "$copy"' EXIT
trap 'exit 143' TERM
trap 'exit 130' INT
if command -v flock >/dev/null && [ -f "$conf.lock" ]; then
    flock --shared "$conf.lock" cp -a "$tokendir" "$copy/tokens"
else
    cp -a "$tokendir" "$copy/tokens"
fi
{
    sed '/^[[:space:]]*directories\.tokendir/d' "$conf"
    echo "directories.tokendir = $copy/tokens"
} >"$copy/softhsm2.conf"
SOFTHSM2_CONF="$copy/softhsm2.conf" "$@"
