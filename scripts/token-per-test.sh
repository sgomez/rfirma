#!/usr/bin/env bash
#
# Envoltorio de nextest: cada proceso de prueba firma contra su propia copia del
# almacen de SoftHSM, porque C_Login reescribe token.object y otro proceso que lo
# relea a la vez pierde la ranura (CKR_GENERAL_ERROR, CKR_SLOT_ID_INVALID).

set -euo pipefail

conf="${SOFTHSM2_CONF:-$HOME/.config/softhsm2/softhsm2.conf}"
[ -f "$conf" ] || conf=/etc/softhsm/softhsm2.conf
tokendir="$(sed -n 's/^[[:space:]]*directories\.tokendir[[:space:]]*=[[:space:]]*//p' "$conf" 2>/dev/null || true)"
if [ -z "$tokendir" ] || [ ! -d "$tokendir" ]; then
    exec "$@"
fi

copy="$(mktemp -d "${TMPDIR:-/tmp}/rfirma-token.XXXXXX")"
trap 'rm -rf "$copy"' EXIT
cp -a "$tokendir" "$copy/tokens"
{
    sed '/^[[:space:]]*directories\.tokendir/d' "$conf"
    echo "directories.tokendir = $copy/tokens"
} >"$copy/softhsm2.conf"
SOFTHSM2_CONF="$copy/softhsm2.conf" "$@"
