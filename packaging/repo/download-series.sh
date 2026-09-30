#!/usr/bin/env bash
# Baja y verifica, versión a versión, toda la serie menor publicada de una etiqueta, sin candidatas (ADR-0015).
#
# Uso: download-series.sh <etiqueta> <serie>
#   <etiqueta>  la etiqueta que se esta publicando, p. ej. v0.4.2
#   <serie>     directorio de salida; SE BORRA Y SE REHACE
# Necesita GH_TOKEN y la clave publica de rFirma en el llavero.
set -euo pipefail

if [ "$#" -ne 2 ]; then
    echo "uso: download-series.sh <etiqueta> <serie>" >&2
    exit 2
fi

etiqueta="$1"
serie="${2%/}"

if ! [[ "$etiqueta" =~ ^(v[0-9]+\.[0-9]+)\. ]]; then
    echo "la etiqueta '$etiqueta' no tiene la forma vMAYOR.MENOR.PARCHE" >&2
    exit 1
fi
prefijo="${BASH_REMATCH[1]}."

verifica="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/verify-packages.sh"

rm -rf "$serie"
mkdir -p "$serie"

# `--exclude-drafts` deja pasar las candidatas: no son borradores.
mapfile -t versiones < <(
    gh release list --limit 200 --exclude-drafts \
        --json tagName,isPrerelease,isDraft \
        --jq '.[] | select(.isPrerelease | not) | select(.isDraft | not) | .tagName' \
        | grep "^${prefijo//./\\.}" | sort -V
)

if [ "${#versiones[@]}" -eq 0 ]; then
    echo "no hay ninguna Release publicada de la serie $prefijo*" >&2
    exit 1
fi

for version in "${versiones[@]}"; do
    echo "  $version"
    gh release download "$version" --dir "$serie/$version"
    "$verifica" "$serie/$version" --signed
    rm -f "$serie/$version/manual-gate.pdf"
done

echo "OK  serie $prefijo* descargada y verificada: ${versiones[*]}"
